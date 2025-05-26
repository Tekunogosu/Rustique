use crate::aliases::{ModFileName, ModID, ModName, ModVersion};
use crate::api::api_structs::{Mod, ModsSearchFile};
use crate::api::client::{ApiClient};
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_all_mods_metadata, find_mod_id, get_current_time, parse_json_file, timestamp_older_than, write_json_file};
use crate::version_management::{parse_latest_version, parse_pinned_version, parse_version};
use comfy_table::{Attribute, Color};
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::collections::HashMap;
use std::default::Default;
use std::path::PathBuf;
use std::time::{Instant};
use comfy_table::presets::UTF8_HORIZONTAL_ONLY;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};
use owo_colors::OwoColorize;
use crate::config::config_manager::{get_config, Config, Package};
use crate::consts::{FILE_GAME_VERSION_SYNC, FILE_MOD_SEARCH_SYNC, FILE_RUSTIQUE_SYNC};
use crate::information_utils::{display_table, elapsed_footer, notice, CellData};
use crate::modpack::symlink_manager::SymlinkManager;
use crate::traits::ref_ext::{PathRef, StrRef};

#[derive(Deserialize, Serialize, Debug)]
pub struct RustiqueSyncJson {
    #[serde(rename = "RustiqueSync")]
    pub rustique_sync: HashMap<ModID, ModSyncInfo>,
    pub last_sync: String,
}

impl RustiqueSyncJson {
    pub fn new() -> RustiqueSyncJson {
        Self {
            rustique_sync: HashMap::<ModID, ModSyncInfo>::new(),
            last_sync: get_current_time(),
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
    pub game_versions: Vec<String>,
    
    #[serde(default)]
    pub is_symlink: bool
}


#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct GameVersionSync {
    pub game_versions: Vec<String>,
    pub last_sync: String,
}

impl GameVersionSync {
    pub fn new() -> GameVersionSync {
        Self::default()
    }
}


// put pinned version in config file, sync checks for pinned versions
// and responds accordingly
// list will still show latest version but with (pinned @v0.2.3) with different text color



// This contains all the data from the api/mods request. This is used to located mod_IDs
// and for searching for new mods. This is synced once a day or manually
pub async fn get_sync_data(mod_dir: impl PathRef) -> Result<RustiqueSyncJson, RustiqueError> {
    let mod_dir = mod_dir.as_ref();
    let fp = mod_dir.join(PathBuf::from(FILE_RUSTIQUE_SYNC));
    if !fp.exists() {
        sync(mod_dir, false, vec![]).await?;
    }

    parse_json_file::<RustiqueSyncJson>(&fp)
}



pub async fn sync<V: AsRef<[Package]>>(mod_dir: impl PathRef, quiet: bool, pin_versions: V) -> Result<(), RustiqueError> {
    let mod_dir = mod_dir.as_ref();
    let start_time = Instant::now();
    let config = get_config().read().await;
    daily_file_syncs(false).await?;
    game_version_sync(false).await?;

    // notice(format!("Syncing {}...", mod_dir.display().fg::<Magenta>()), Option::from(comfy_table::Color::Yellow), vec![Attribute::Bold]);
    display_table(vec![(
        CellData::new("Syncing...".into(), Some(Color::Yellow), vec![Attribute::Bold], None),
        CellData::new(mod_dir.to_string_lossy().to_string(), Some(Color::Magenta), vec![], None)
    )], Some(UTF8_HORIZONTAL_ONLY));
    

    // check if rustique-sync.json exists
    // if so, parse the file for updating
    // if not, do all the sync process and then write a new file
    let sync_file_path = mod_dir.join(FILE_RUSTIQUE_SYNC);
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

    let mods_search_file = config_path.join(FILE_MOD_SEARCH_SYNC);
    let mods_search_data = match parse_json_file::<ModsSearchFile>(&mods_search_file) {
        Ok(json) => json,
        Err(e) => {
            // this shouldn't fail, but if it does, rerun the mod_id_sync()
            info!("mods search sync Error: {}", e.to_string());
            daily_file_syncs(true).await?
        }
    };

    let search_sync_time = i64::from(config.sync_mod_search_file_every);
    
    if timestamp_older_than(search_sync_time, &mods_search_data.last_sync) {
        // update the database
        daily_file_syncs(true).await?;
    }
    
    let game_version_sync_file = config_path.join(FILE_GAME_VERSION_SYNC);
    let game_version_sync_data = match parse_json_file::<GameVersionSync>(&game_version_sync_file) {
        Ok(json) => json,
        Err(e) => {
            info!("game version sync Error: {e}");
            game_version_sync(true).await?
        }
    };
    
    let game_version_time = i64::from(config.sync_latest_game_version_file_every);
    if timestamp_older_than(game_version_time, &game_version_sync_data.last_sync) {
        // update the database
        game_version_sync(true).await?;
    }
     

    let installed_mods = extract_all_mods_metadata(mod_dir, false).await?;

    // clean sync data first so latest info takes priority
    sync_data.rustique_sync.clear();

    for (mod_filename, mod_info) in &installed_mods {
        
        // check if the file is a symlink
        
        let version = if let Ok(parsed_version) = parse_version(&mod_info.version.clone().unwrap_or_default()) {
            parsed_version.to_string()
        } else {
            warn!("Could not parse version: {} for {}\n\rThis mod may not update correctly..", mod_info.version.clone().unwrap_or_default(), mod_filename.to_string());
            mod_info.version.clone().unwrap_or_default()
        };

        info!("{} {} for {}", "VERSION Parsed:".green(), version.magenta(), mod_info.mod_id.yellow());

        // check here for bad mod_id
        let mod_id = if mod_info.mod_id.is_empty() {
            find_mod_id(&mod_info.name, mod_filename, &mods_search_data.mods)?
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
                game_versions: Vec::new(),
                is_symlink: SymlinkManager::exists(mod_dir.join(mod_filename)),
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
    
    for (mod_id, res_mod) in &result {

        let pkg = if pin_versions.as_ref().is_empty() {
            config.pkg.iter().find(|p| p.mod_id.eq(mod_id)).cloned().unwrap_or_default()
        } else {
            pin_versions.as_ref().iter().find(|p| p.mod_id.eq(mod_id)).cloned().unwrap_or_default()
        };
        
        let (mod_version, download_url, game_versions) = if !pkg.mod_id.is_empty() || !config.pinned_game_version.is_empty() {
            parse_pinned_version(&res_mod.mod_json.releases, &pkg, config.pinned_game_version.clone())
        } else {
            parse_latest_version(&res_mod.mod_json.releases)
        };

        sync_data
            .rustique_sync
            .entry(mod_id.clone())
            .and_modify(|sync_info| {
                sync_info.latest_known_version.clone_from(&mod_version);
                sync_info.latest_download_url.clone_from(&download_url);
                sync_info.game_versions.clone_from(&game_versions);
            })
            .or_insert_with(|| ModSyncInfo {
                latest_known_version: mod_version,
                latest_download_url: download_url,
                mod_name: res_mod.mod_json.name.clone().unwrap_or_default(),
                game_versions,
                .. Default::default()
            });
    }

    // Write the sync data to file
    let data = sync_data;
    let json = prettify(&data, "Sync")?;

    // Use tokio's async file operations
    let mut file = File::create(sync_file_path)
        .await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Error writing sync file to mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;

    AsyncWriteExt::write_all(&mut file, json.as_bytes()).await?;

    if config.show_execution_time && !quiet {
        elapsed_footer(start_time, "Sync");
    }

    Ok(())
}


pub async fn daily_file_syncs(force: bool) -> Result<ModsSearchFile, RustiqueError> {

    let config = get_config().read().await;
    let start_time = Instant::now();

    let config_dir = Config::get_path();

    // Sync game versions

    let search_file = config_dir.join(FILE_MOD_SEARCH_SYNC);
    info!("{} {}","Search file path:".green(), search_file.to_string_lossy().yellow());

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

    let sync_time = i64::from(config.sync_mod_search_file_every);

    if file_data.mods.is_empty() || force || timestamp_older_than(sync_time, &file_data.last_sync){

        notice("Daily Search Sync...", Some(Color::Yellow), vec![Attribute::Bold]);

        let client = ApiClient::new();
        // get all mod info
        file_data.mods = client.fetch_all_mods().await?.mods;
        file_data.last_sync = get_current_time();

        debug!("file_data {:?}", file_data);

        info!("Attempting to write Mod Search file to {}", search_file.display());

        let json = prettify(&file_data, "Mods Search DB")?;
        write_json_file(&search_file, json, &Config::get_path()).await?;
        
        info!("Mods Search Sync file written successfully");
    }

    if config.show_execution_time && force {
        elapsed_footer(start_time, "Mods Search Sync");
    }

    Ok(file_data)
}

pub async fn game_version_sync(force: bool) -> Result<GameVersionSync, RustiqueError> {
  
    let start_time = Instant::now();
    let config = get_config().read().await;
    
    let file = Config::get_path().join(FILE_GAME_VERSION_SYNC);
    info!("{} {}","Game version sync file path:".green(), file.to_string_lossy().yellow());
    // if the file doesn't exit, create it 
    // otherwise check if its time to do update
    
    let mut file_data = if file.exists() {
        match parse_json_file::<GameVersionSync>(&file) {
            Ok(json) => json,
            Err(e) => {
                info!("Game version sync file parse error: {}", e);
                // delete the file and recreate it
                tokio::fs::remove_file(&file).await?;
                GameVersionSync::new()
            }
        }
    } else {
        GameVersionSync::new()
    };
    
    let sync_time = i64::from(config.sync_latest_game_version_file_every);
    
    if file_data.game_versions.is_empty() || force || timestamp_older_than(sync_time, &file_data.last_sync){
        notice("Syncing latest game versions..", Some(comfy_table::Color::Yellow), vec![Attribute::Bold]);
        
        let client = ApiClient::new();
        let gvs = client.fetch_game_versions().await?;
        file_data.game_versions = gvs.into_iter().collect();
        file_data.last_sync = get_current_time();
        
        let json = prettify(&file_data, "Game Version Sync")?;
        
        write_json_file(&file, json, &Config::get_path()).await?;

        info!("Mods Search Sync file written successfully");
        
    }
    
    
     if config.show_execution_time && force {
        elapsed_footer(start_time, "Game Version Sync");
    } 
    
    Ok(file_data)
}

pub fn prettify<T>(data: T, command_type: impl StrRef) -> Result<String, RustiqueError>
    where
    T: serde::Serialize {

    to_string_pretty(&data).map_err(|e| RustiqueError::JsonError {
            context: format!("Failure while making the {} json pretty", command_type.as_ref()),
            source: serde_json5::Error::from(std::io::Error::other(e)),
        })
}
