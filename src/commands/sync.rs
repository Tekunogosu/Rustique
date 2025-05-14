use crate::aliases::{ModFileName, ModID, ModName, ModVersion};
use crate::api::api_structs::{Mod, ModApi, ModsSearchFile};
use crate::api::client::{ApiClient};
use crate::config_manager::{get_config, Config};
use crate::rustique_errors::RustiqueError;
use crate::utils::{delete_file, elapsed_footer, extract_all_mods_metadata, get_current_time, notice, timestamp_older_than};
use crate::version_management::{parse_latest_version, parse_version};
use owo_colors::OwoColorize;
use comfy_table::Attribute;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read};
use std::path::PathBuf;
use std::process::exit;
use std::time::{Instant};
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

#[derive(Deserialize, Serialize, Debug)]
pub struct RustiqueSyncJson {
    #[serde(rename = "RustiqueSync")]
    pub rustique_sync: HashMap<ModID, ModSyncInfo>,
    pub last_sync: String,
    pub last_modid_sync: String,
}

impl RustiqueSyncJson {
    pub fn new() -> RustiqueSyncJson {
        Self {
            rustique_sync: HashMap::<ModID, ModSyncInfo>::new(),
            last_sync: get_current_time(),
            last_modid_sync: get_current_time(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ModIDSync {
    pub all_mods: HashMap<ModName, ModIDSyncData>,
    pub last_sync: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModIDSyncData {
    pub mod_id: ModID,
    pub modid_strs: Vec<String>
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ModSyncInfo {
    pub file_name: ModFileName,
    pub mod_name: String,
    pub installed_version: ModVersion,
    pub latest_known_version: ModVersion,
    pub latest_download_url: String,
}

#[allow(unused)]
pub async fn handle_sync_call(mod_dir: &PathBuf) {
    match sync(mod_dir).await {
        Ok(()) => {}
        Err(e) => {
           error!("{}", e.to_string());
            exit(1);
        }
    }
}

pub const SYNC_FILE_NAME: &str = "rustique-sync.json";
pub const MODID_SYNC_FILE_NAME: &str = "mod-id-sync.json";

// This contains all the data from the api/mods request. This is used to located mod_IDs
// and for searching for new mods. This is synced once a day or manually
pub const SEARCH_FILE_NAME: &str = "mod-search.json";


pub fn parse_json_file<T>(file_path: &PathBuf) -> Result<T, RustiqueError>
where
    T: for<'de> serde::Deserialize<'de>
{
    let filename = file_path.file_name().unwrap().to_string_lossy().to_string();

    let mut file = File::open(file_path).map_err(|e| RustiqueError::IoError {
        context: format!("Unable to open {filename}"),
        source: e,
    })?;

    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents).map_err(|e| RustiqueError::IoError {
        context: format!("Failure while reading from file {filename}"),
        source: e
    })?;

    let json = serde_json5::from_str::<T>(&file_contents)
        .map_err(|e| RustiqueError::JsonError {
            context: format!("Json parsing Error for {filename}"),
            source: e
        })?;

    Ok(json)
}

pub async fn get_sync_data(mod_dir: &PathBuf) -> Result<RustiqueSyncJson, RustiqueError> {

    let fp = mod_dir.join(PathBuf::from(SYNC_FILE_NAME));
    if !fp.exists() {
        sync(mod_dir).await?;
    }

    parse_json_file::<RustiqueSyncJson>(&fp)
}



pub async fn sync(mod_dir: &PathBuf) -> Result<(), RustiqueError> {

    let start_time = Instant::now();
    let config = get_config().read().await;
    mods_search_sync(false).await?;

    notice("Syncing...", Option::from(comfy_table::Color::Yellow), vec![Attribute::Bold]);

    // check if rustique-sync.json exists
    // if so, parse the file for updating
    // if not, do all the sync process and then write a new file
    let sync_file_path = mod_dir.join(SYNC_FILE_NAME);
    debug!("sync file: {}", sync_file_path.display());

    let mut sync_data = if sync_file_path.exists() {
        match parse_json_file::<RustiqueSyncJson>(&sync_file_path) {
            Ok(json) => json,
            Err(e) => {
                info!("{}",e);
                // delete the sync file because the json changed
                tokio::fs::remove_file(&sync_file_path).await?;
                // return a blank slate to keep going
                RustiqueSyncJson::new()
            }
        }
    } else {
       RustiqueSyncJson::new()
    };

    let config_path = Config::get_path();

    let mods_search_file = config_path.join(SEARCH_FILE_NAME);
    let mods_search_data = match parse_json_file::<ModsSearchFile>(&mods_search_file) {
        Ok(json) => json,
        Err(e) => {
            // this shouldn't fail, but if it does, rerun the mod_id_sync()
            info!("mods search sync Error: {}", e.to_string());
            mods_search_sync(true).await?
        }
    };

    if timestamp_older_than(24, &mods_search_data.last_sync) {
        // update the database
        mods_search_sync(true).await?;
    }

    let installed_mods = extract_all_mods_metadata(mod_dir).await?;

    // clean sync data first so latest info takes priority
    sync_data.rustique_sync.clear();

    for (mod_filename, mod_info) in &installed_mods {
        let version = if let Ok(parsed_version) = parse_version(&mod_info.version.clone().unwrap_or_default()) {
            parsed_version.to_string()
        } else {
            warn!("Could not parse version: {} for {}\n\rThis mod may not update correctly..", mod_info.version.clone().unwrap_or_default(), mod_filename.to_string());
            mod_info.version.clone().unwrap_or_default()
        };

        info!("VERSION Parsed: {} for {}", version, mod_info.mod_id);

        // check here for bad mod_id
        let mod_id = if mod_info.mod_id.is_empty() {
            info!("{} has an empty mod id, attempting locate mod id...", mod_filename);
            let res: Vec<ModApi> = mods_search_data.mods.iter().filter(|mod_search| {
                match &mod_search.name {
                    Some(name) => {
                        mod_info.name.to_lowercase().eq(&name.to_lowercase())
                    }
                    None => {
                        mod_search.mod_id_strs.contains(&mod_info.name)
                    }
                }
            }).cloned().collect();

            if res.is_empty() || res.len() > 1 {
                // no mods match
                warn!("Unable to determine the mod_id for {} - {}.\n\r\t Their modinfo.json is malformed and no information provided allowed Rustique to determine it.\n\r\t \
                             Please contact the author to correct their modinfo.json file", mod_info.name.bright_red().bold(), mod_filename.bright_red().bold());
                String::new()
            } else {
                res[0].mod_id.to_string()
            }

        } else {
            mod_info.mod_id.clone()
        };

        sync_data
            .rustique_sync
            .entry(mod_id)
            .or_insert_with(|| ModSyncInfo {
                installed_version: version.clone(),
                file_name: mod_filename.clone(),
                mod_name: mod_info.name.clone(),
                latest_download_url: String::new(),
                latest_known_version: String::new(),
            });
    }

    let im = installed_mods.keys().clone().collect::<Vec<&String>>();
    info!("Installed mods: {:?}", im);

    // Create API client and fetch mods in parallel using async
    let client = ApiClient::new();
    let result: HashMap<ModID, Mod> = client
        .fetch_mods_parallel(
            sync_data.rustique_sync.keys().cloned().collect()
        ).await?;

    for (mod_id, mod_info) in &result {
        let (mod_version, download_url) = parse_latest_version(&mod_info.mod_json.releases);

        sync_data
            .rustique_sync
            .entry(mod_id.clone())
            .and_modify(|sync_info| {
                sync_info.latest_known_version.clone_from(&mod_version);
                sync_info.latest_download_url.clone_from(&download_url);
            })
            .or_insert_with(|| ModSyncInfo {
                latest_known_version: mod_version,
                latest_download_url: download_url,
                mod_name: mod_info.mod_json.name.clone().unwrap_or_default(),
                file_name: "None".to_string(),
                installed_version: "None".to_string(),
            });
    }

    // Write the sync data to file
    let data = sync_data;
    let json = prettify(&data, "Sync")?;

    // Use tokio's async file operations
    let mut file = tokio::fs::File::create(sync_file_path)
        .await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Error writing sync file to mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;

    AsyncWriteExt::write_all(&mut file, json.as_bytes()).await?;

    if config.show_execution_time {
        elapsed_footer(start_time, "Sync");
    }

    Ok(())
}

pub async fn mods_search_sync(force: bool) -> Result<ModsSearchFile, RustiqueError> {

    let config = get_config().read().await;
    let start_time = Instant::now();

    let config_dir = Config::get_path();

    let mod_id_sync = config_dir.clone().join(MODID_SYNC_FILE_NAME);
    if mod_id_sync.exists() {
        delete_file(&mod_id_sync).await?;
    }

    let search_file = config_dir.join(SEARCH_FILE_NAME);
    info!("Search file path: {}", search_file.to_string_lossy());

    let mut file_data = if search_file.exists() {
        match parse_json_file::<ModsSearchFile>(&search_file) {
            Ok(json) => json,
            Err(e) => {
                // delete the file and try again
                info!("mod-search.json parse error: {}", e);
                tokio::fs::remove_file(&search_file).await?;
                // println!("mod-search.json parse error: {}", e);
                ModsSearchFile::new()
            }
        }
    } else {
        ModsSearchFile::new()
    };


    if file_data.mods.is_empty() || force || timestamp_older_than(24, &file_data.last_sync){

        notice("Daily Search Sync...", Some(comfy_table::Color::Yellow), vec![Attribute::Bold]);

        let client = ApiClient::new();
        // get all mod info
        file_data.mods = client.fetch_all_mods().await?.mods;
        file_data.last_sync = get_current_time();

        debug!("file_data {:?}", file_data);

        info!("Attempting to write Mod Search file to {}", search_file.display());

        let json = prettify(&file_data, "Mods Search DB")?;
        let mut open_file = tokio::fs::File::create(search_file).await.map_err(|e| RustiqueError::IoError {
            context: format!("Error writing sync mod search file to config dir: {}", config_dir.to_string_lossy()),
            source: e,
        })?;
        AsyncWriteExt::write_all(&mut open_file, json.as_bytes()).await?;

        info!("Mods Search Sync file written successfully");
    }

    if config.show_execution_time && force {
        elapsed_footer(start_time, "Mods Search Sync");
    }

    Ok(file_data)
}

pub fn prettify<T>(data: T, command_type: &str) -> Result<String, RustiqueError>
    where
    T: serde::Serialize {

    to_string_pretty(&data).map_err(|e| RustiqueError::JsonError {
            context: format!("Failure while making the {command_type} json pretty"),
            source: serde_json5::Error::from(std::io::Error::other(e)),
        })
}
