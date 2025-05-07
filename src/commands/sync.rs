use std::collections::HashMap;
use std::error::Error;
use std::fmt::Error;
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use colored::{Color, Colorize};
use comfy_table::Attribute;
use rayon::prelude::*;
use serde_json::to_string_pretty;
use ureq::Agent;
use semver::{Error, Version};
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};
use crate::aliases::{ModFileName, ModID, ModIDInt, ModName, ModVersion};
use crate::rustique_errors::RustiqueError;
use crate::api::api_structs::{Mod, ModInfo, Releases};
use crate::utils::{RustiqueOptions, get_current_time, extract_all_mods_metadata, elapsed_footer, notice, is_today, get_expanded_path, timestamp_older_than};
use crate::api::client::{ApiClient, ModApiFetch};
use crate::config_manager::{get_config, CONFIG_DEFAULT_DIR};
use crate::install_manager::Install;
use crate::rustique_errors::RustiqueError::UrlParseError;
use crate::version_management::{parse_latest_version, parse_version};

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
impl ModIDSync {
    pub fn new() -> ModIDSync {
        Self {
            all_mods: HashMap::new(),
            last_sync: String::new(),
        }
    }
}



#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModSyncInfo {
    pub file_name: ModFileName,
    pub mod_name: String,
    pub installed_version: ModVersion,
    pub latest_known_version: ModVersion,
    pub latest_download_url: String,
}


pub async fn handle_sync_call(mod_dir: &PathBuf) {
    match sync(mod_dir).await {
        Ok(_) => {}
        Err(e) => {
           error!("{}", e.to_string());
            exit(1);
        }
    }
}

pub const SYNC_FILE_NAME: &str = "rustique-sync.json";
pub const MODID_SYNC_FILE_NAME: &str = "mod-id-sync.json";


pub fn parse_json_file<T>(file_path: &PathBuf) -> Result<T, RustiqueError>
where
    T: for<'de> serde::Deserialize<'de>
{
    let filename = file_path.file_name().unwrap().to_string_lossy().to_string();

    let mut file = File::open(file_path).map_err(|e| RustiqueError::IoError {
        context: format!("Unable to open {}", filename),
        source: e,
    })?;

    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents).map_err(|e| RustiqueError::IoError {
        context: format!("Failure while reading from file {}", filename),
        source: e
    })?;

    let json = serde_json5::from_str::<T>(&file_contents)
        .map_err(|e| RustiqueError::JsonError {
            context: format!("Json parsing Error for {}", filename),
            source: e
        })?;

    Ok(json)
}

pub fn get_sync_data(mod_dir: &PathBuf) -> Result<RustiqueSyncJson, RustiqueError> {

    let fp = mod_dir.join(PathBuf::from(SYNC_FILE_NAME));
    if !fp.exists() {
        sync(mod_dir)?;
    }


    Ok(parse_json_file::<RustiqueSyncJson>(&fp).map_err(|e| {
        RustiqueError::JsonError {
            context: format!("Failed to parse json file {}", fp.to_string_lossy()),
            source: serde_json5::Error::from(format!("{}", e.to_string())),
        }
    }))?
}



pub async fn sync(mod_dir: &PathBuf) -> Result<(), RustiqueError> {

    let start_time = Instant::now();
    let config = get_config().read().unwrap();
    mod_id_sync(false).await?;

    notice("Syncing...", Option::from(comfy_table::Color::Blue), vec![Attribute::Bold]);

    // check if rustique-sync.json exists
    // if so, parse the file for updating
    // if not, do all the sync process and then write a new file
    let sync_file_path = mod_dir.join(SYNC_FILE_NAME);
    debug!("sync file: {}", sync_file_path.display());

    let mut sync_data = if sync_file_path.exists() {
        match parse_json_file::<RustiqueSyncJson>(&sync_file_path) {
            Ok(json) => json,
            Err(e) => {
                // delete the sync file because the json changed
                tokio::fs::remove_file(&sync_file_path).await?;

                // return a blank slate to keep going
                RustiqueSyncJson::new()
            }
        }
    } else {
       RustiqueSyncJson::new()
    };

    if timestamp_older_than(24, &sync_data.last_modid_sync) {
        // update the database
        mod_id_sync(true).await?;
        sync_data.last_modid_sync = get_current_time();
    }

    let mod_id_fp = get_expanded_path(PathBuf::from(CONFIG_DEFAULT_DIR).join(MODID_SYNC_FILE_NAME));
    let id_sync_data = match parse_json_file::<ModIDSync>(&mod_id_fp) {
        Ok(json) => json,
        Err(e) => {
            // this shouldn't fail, but if it does, rerun the mod_id_sync()
            info!("mod id sync Error: {}", e.to_string());
            mod_id_sync(true).await?
        }
    };

    let installed_mods = extract_all_mods_metadata(mod_dir)?;

    // clean sync data first so latest info takes priority
    sync_data.rustique_sync.clear();

    installed_mods.iter().for_each(|(mod_filename, mod_info)| {
        let version = if let Ok(parsed_version) = parse_version(mod_info.version.clone().unwrap_or_default()) {
            parsed_version.to_string()
        } else {
            warn!("Could not parse version: {} for {}\n\rThis mod may not update correctly..", mod_info.version.clone().unwrap_or_default(), mod_filename.to_string());
            mod_info.version.clone().unwrap_or_default()
        };

        info!("VERSION Parsed: {} for {}", version, mod_info.mod_id);

        // check here for bad mod_id
        let mod_id = if mod_info.mod_id.is_empty() {
            info!("{} has an empty mod id, attempting find it...", mod_filename);
            match id_sync_data.all_mods.get(&mod_info.name) {
                Some(id) => id.mod_id.clone(),
                None => {
                    match id_sync_data.all_mods.values().find(|mod_data| mod_data.modid_strs.contains(&mod_info.name)) {

                        Some(id) => id.mod_id.clone(),
                        None => {
                            warn!("Unable to determine the mod_id for {} - {}.\n\r\t Their modinfo.json is malformed and no information provided allowed Rustique to determine it.\n\r\t \
                             Please contact the author to correct their modinfo.json file", mod_info.name.bright_red().bold(), mod_filename.bright_red().bold());
                            // We let the value be blank, rustique will report the issue again, but continue
                            "".to_string()
                        }
                    }
                }
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
    });

    let im = installed_mods.keys().clone().collect::<Vec<&String>>();
    info!("Installed mods: {:?}", im);

    // Create API client and fetch mods in parallel using async
    let client = ApiClient::new();
    let result: HashMap<ModID, Mod> = client
        .fetch_mods_parallel(
            sync_data.rustique_sync.keys().cloned().collect()
        ).await?;

    result.iter().for_each(|(mod_id, mod_info): (&ModID, &Mod)| {
        let (mod_version, download_url) = parse_latest_version(&mod_info.mod_json.releases);

        sync_data
            .rustique_sync
            .entry(mod_id.clone())
            .and_modify(|sync_info| {
                sync_info.latest_known_version = mod_version.clone();
                sync_info.latest_download_url = download_url.clone();
            })
            .or_insert_with(|| ModSyncInfo {
                latest_known_version: mod_version,
                latest_download_url: download_url,
                mod_name: mod_info.mod_json.name.clone().unwrap_or_default(),
                file_name: "None".to_string(),
                installed_version: "None".to_string(),
            });
    });

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

    AsyncWriteExt::write_all(&mut file, json.as_bytes())
        .await?;
        // .map_err(|e| RustiqueError::ApiError {
        //     context: format!("Error writing data to sync file: {}", file_path.to_string_lossy()),
        //     source: e,
        // })?;

    if config.show_execution_time {
        elapsed_footer(start_time, "Sync");
    }

    Ok(())
}

pub async fn mod_id_sync(force: bool) -> Result<ModIDSync, RustiqueError> {


    let config = get_config().read().unwrap();
    let start_time = Instant::now();

    let config_dir = get_expanded_path(PathBuf::from(CONFIG_DEFAULT_DIR));
    let file_path = config_dir.join(MODID_SYNC_FILE_NAME);

    let mut file_data = if file_path.exists() {
        match parse_json_file::<ModIDSync>(&file_path) {
            Ok(json) => json,
            Err(e) => {
                // delete the file and try again
                info!("mod_id_sync json parse error: {}", e);
                tokio::fs::remove_file(&file_path).await?;
                println!("mod_id_sync json parse error: {}", e);
                ModIDSync::new()
            }
        }
    } else {
        ModIDSync {
            all_mods: HashMap::new(),
            last_sync: get_current_time(),
        }
    };

    //
    if file_data.all_mods.is_empty() || force || timestamp_older_than(24, &file_data.last_sync){

        notice("Daily ModID Sync...", Some(comfy_table::Color::Blue), vec![Attribute::Bold]);

        let client = ApiClient::new();
        // get all mod info
        let all_mods = client.fetch_all_mods().await?;

        file_data.all_mods = all_mods.mods
            .into_iter()
            .filter_map(|m| {
                Some((m.name.unwrap_or_default(), ModIDSyncData {
                    mod_id: m.mod_id.to_string(),
                    modid_strs: m.mod_id_strs,
                }))
            })
            .collect();

        debug!("file_data {:?}", file_data);

        info!("Attempting to write ModID Sync to {}", file_path.display());

        let json = prettify(&file_data, "Sync ModID")?;
        let mut open_file = tokio::fs::File::create(file_path).await.map_err(|e| RustiqueError::IoError {
            context: format!("Error writing sync file to config dir: {}", config_dir.to_string_lossy()),
            source: e,
        })?;;
        AsyncWriteExt::write_all(&mut open_file, json.as_bytes()).await?;

        info!("ModID Sync file written successfully");
    }

    if config.show_execution_time && force {
        elapsed_footer(start_time, "ModID Sync");
    }

    Ok(file_data)
}

pub fn prettify<T>(data: T, command_type: &str) -> Result<String, RustiqueError>
    where
    T: serde::Serialize {

    to_string_pretty(&data).map_err(|e| RustiqueError::JsonError {
            context: format!("Failure while making the {} json pretty", command_type),
            source: serde_json5::Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)),
        })
}
