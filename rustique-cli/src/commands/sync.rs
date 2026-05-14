
use comfy_table::{Attribute, Color};
use std::collections::HashMap;
use std::default::Default;
use std::path::PathBuf;
use std::time::{Instant};
use comfy_table::presets::UTF8_HORIZONTAL_ONLY;
use tracing::{debug, error, info, warn};
use owo_colors::OwoColorize;
use rustique_core::aliases::{ModID, PinnedVersionInfo};
use rustique_core::config::config_manager::{get_config, Config, Package};
use rustique_core::consts::{FILE_GAME_VERSION_SYNC, FILE_MOD_SEARCH_SYNC, FILE_RUSTIQUE_SYNC};
use rustique_core::information_utils::{display_incompatible_mods_constraint, display_table, elapsed_footer, notice, CellData};
use rustique_core::symlink_manager::SymlinkManager;
use rustique_core::traits::ref_ext::{PathRef};
use rustique_core::api::api_structs::{Mod, ModsSearchFile};
use rustique_core::api::client::{ApiClient};
use rustique_core::rustique_errors::RustiqueError;
use rustique_core::sync_structs::{GameVersionSync, ModSyncInfo, RustiqueSyncJson};
use rustique_core::utils::{extract_all_mods_metadata, find_mod_id, get_current_time, split_modid_version, parse_json_file, timestamp_older_than, write_json_file, prettify};
use rustique_core::version_management::{parse_latest_version, parse_pinned_version, parse_version};


/// Use this function to retrieve the sync file for mod_dir. 
pub async fn get_sync_data(mod_dir: impl PathRef, quiet: bool) -> Result<RustiqueSyncJson, RustiqueError> {
    let mod_dir = mod_dir.as_ref();
    let fp = mod_dir.join(PathBuf::from(FILE_RUSTIQUE_SYNC));
    if !fp.exists() {
        info!("Sync file doesn't exist, running sync");
        sync(mod_dir, quiet, vec![]).await?;
    }
    
    info!("Sync file located: {fp:?}");

    parse_json_file::<RustiqueSyncJson>(&fp).await
}

pub async fn sync<V: AsRef<[Package]>>(mod_dir: impl PathRef, quiet: bool, pin_versions: V) -> Result<RustiqueSyncJson, RustiqueError> {
    let mod_dir = mod_dir.as_ref();
    let start_time = Instant::now();
    let config = get_config().read().await;
    daily_file_syncs(false).await?;
    game_version_sync(false).await?;

    // notice(format!("Syncing {}...", mod_dir.display().fg::<Magenta>()), Option::from(comfy_table::Color::Yellow), vec![Attribute::Bold]);
    if !quiet {
        display_table(vec![(
            CellData::new("Syncing...".into(), Some(Color::Yellow), vec![Attribute::Bold], None),
            CellData::new(mod_dir.to_string_lossy().to_string(), Some(Color::Magenta), vec![], None)
        )], Some(UTF8_HORIZONTAL_ONLY));
    }
    

    // This is THE section that creates a new sync file if one does not exist. 
    // All functions should call get_sync_data() instead of checking for sync_file manually
    //
    // check if rustique-sync.json exists
    // if so, parse the file for updating
    // if not, do all the sync process and then write a new file
    let sync_file_path = mod_dir.join(FILE_RUSTIQUE_SYNC);
    debug!("sync file: {}", sync_file_path.display());

    let mut sync_data = if sync_file_path.exists() {
        match parse_json_file::<RustiqueSyncJson>(&sync_file_path).await {
            Ok(json) => json,
            Err(e) => {
                info!("{}",e);
                // delete the sync file because the json changed
                tokio::fs::remove_file(&sync_file_path).await?;
                // return a blank slate to keep going
                RustiqueSyncJson::default()
            }
        }
    } else {
       RustiqueSyncJson::default()
    };

    let config_path = Config::get_path();

    let mods_search_file = config_path.join(FILE_MOD_SEARCH_SYNC);
    let mods_search_data = match parse_json_file::<ModsSearchFile>(&mods_search_file).await {
        Ok(json) => json,
        Err(e) => {
            // this shouldn't fail, but if it does, rerun the mod_id_sync()
            info!("mods search sync Error: {}", e.to_string());
            daily_file_syncs(true).await?
        }
    };

    let search_sync_time = config.sync_mod_search_file_every;
    
    if timestamp_older_than(search_sync_time, &mods_search_data.last_sync) {
        // update the database
        daily_file_syncs(true).await?;
    }
    
    let game_version_sync_file = config_path.join(FILE_GAME_VERSION_SYNC);
    let game_version_sync_data = match parse_json_file::<GameVersionSync>(&game_version_sync_file).await {
        Ok(json) => json,
        Err(e) => {
            info!("game version sync Error: {e}");
            game_version_sync(true).await?
        }
    };
    
    let game_version_time = config.sync_latest_game_version_file_every;
    if timestamp_older_than(game_version_time, &game_version_sync_data.last_sync) {
        // update the database
        game_version_sync(true).await?;
    }
     

    let installed_mods = extract_all_mods_metadata(mod_dir, false).await?;

    debug!("INSTALLED_MODS in sync: {:?}", installed_mods);

    // clean sync data first so latest info takes priority
    sync_data.rustique_sync.clear();

    for (mod_filename, mod_info) in &installed_mods {
        
        debug!("MOD_INFO in sync: {:?}", mod_info);

        // check if the file is a symlink
        let version = if let Ok(parsed_version) = parse_version(&mod_info.version.clone().unwrap_or_default()) {
            parsed_version.to_string()
        } else {
            warn!("Could not parse version: {} for {}\n\rThis mod may not update correctly..", mod_info.version.clone().unwrap_or_default(), mod_filename.clone());
            mod_info.version.clone().unwrap_or_default()
        };

        info!("{} {} for {}", "VERSION Parsed:".green(), version.magenta(), mod_info.mod_id.yellow());

        // check here for bad mod_id
        let mod_id = if mod_info.mod_id.is_empty() {
            match find_mod_id(&mod_info.name, mod_filename, &mods_search_data.mods) {
                Ok(mod_id) => mod_id,
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
            }
        } else {
            mod_info.mod_id.clone().to_lowercase()
        };

        sync_data
            .rustique_sync
            .entry(mod_id)
            .or_insert_with(|| ModSyncInfo {
                installed_version: version.clone(),
                file_name: mod_filename.clone(),
                mod_name: mod_info.name.clone(),
                asset_id: 0, // will be updated when we make our api call
                latest_download_url: String::new(),
                latest_known_version: String::new(),
                game_versions: Vec::new(),
                is_symlink: SymlinkManager::exists(mod_dir.join(mod_filename)),
                latest_changelog: String::new(),
            });
    }
    
    info!("Sync data before api call {:#?}", sync_data);

    let im = installed_mods.keys().clone().collect::<Vec<&String>>();
    info!("Installed mods: {:?}", im);

    // Create API client and fetch mods in parallel using async
    let client = ApiClient::new();
    let result: HashMap<ModID, Mod> = client
        .fetch_mods_parallel(
            sync_data.rustique_sync.keys().map(|m|split_modid_version(m).0.clone()).collect()
        ).await?;

    let mut no_compatible_mods: Vec<String> = Vec::new();
    
    for (mod_id, res_mod) in &result {

        // let (mod_id_parsed, _) = &split_modid_version(mod_id);
        // force to lowercase because some authors put uppercase chars in the modid
        let mod_id = mod_id.to_lowercase();
        let mod_asset_id = res_mod.mod_json.asset_id;
        
        let pkg = if pin_versions.as_ref().is_empty() {
            config.pkg.iter().find(|p| p.mod_id.eq(&mod_id)).cloned().unwrap_or_default()
        } else {
            pin_versions.as_ref().iter().find(|p| p.mod_id.eq(&mod_id)).cloned().unwrap_or_default()
        };

        info!("pkg in sync: {:?}", pkg);
        
        let (mod_version, download_url, game_versions, changelog) = if !pkg.mod_id.is_empty() || !config.pinned_game_version.is_empty() {
            info!("{} {}","Parsing pinned versions for".yellow(), mod_id.blue());
            match parse_pinned_version(&res_mod.mod_json.releases, &pkg, config.pinned_game_version.as_str(), config.allow_unstable) {
                Ok(pv) => pv,
                Err(e) => {
                    no_compatible_mods.push(format!("ModID: {mod_id} - AssetID: {mod_asset_id}"));
                    info!("Unable to find compatible version for {mod_id}. {e}");
                    continue
                }
            }
        } else {
            info!("{} {}", "Parsing latest versions for".yellow(), mod_id.blue());
            parse_latest_version(&res_mod.mod_json.releases)
        };

        sync_data
            .rustique_sync
            .entry(mod_id.clone())
            .and_modify(|sync_info| {
                sync_info.latest_known_version.clone_from(&mod_version);
                sync_info.latest_download_url.clone_from(&download_url);
                sync_info.game_versions.clone_from(&game_versions);
                sync_info.latest_changelog.clone_from(&changelog);
                (sync_info.asset_id).clone_from(&mod_asset_id);
            })
            .or_insert_with(|| ModSyncInfo {
                latest_known_version: mod_version,
                latest_download_url: download_url,
                mod_name: res_mod.mod_json.name.clone().unwrap_or_default(),
                asset_id: mod_asset_id,
                game_versions,
                latest_changelog: changelog,
                .. Default::default()
            });
    }

    if !no_compatible_mods.is_empty() {
        display_incompatible_mods_constraint(no_compatible_mods, "Faild sync for mods due to incompatible pinned constraints".into());
    }

    sync_data.save(sync_file_path).await?;
   
    if config.show_execution_time && !quiet {
        elapsed_footer(start_time, "Sync");
    }

    Ok(sync_data)
}


pub async fn daily_file_syncs(force: bool) -> Result<ModsSearchFile, RustiqueError> {

    let config = get_config().read().await;
    let start_time = Instant::now();

    let config_dir = Config::get_path();

    // Sync game versions

    let search_file = config_dir.join(FILE_MOD_SEARCH_SYNC);
    info!("{} {}","Search file path:".green(), search_file.to_string_lossy().yellow());

    let mut file_data = if search_file.exists() {
        match parse_json_file::<ModsSearchFile>(&search_file).await {
            Ok(json) => json,
            Err(e) => {
                // delete the file and try again
                error!("mod-search.json parse error: {}", e);
                tokio::fs::remove_file(&search_file).await?;
                // println!("mod-search.json parse error: {}", e);
                ModsSearchFile::new()
            }
        }
    } else {
        ModsSearchFile::new()
    };

    let sync_time = config.sync_mod_search_file_every;

    if file_data.mods.is_empty() || force || timestamp_older_than(sync_time, &file_data.last_sync){

        notice("Daily Search Sync...", Some(Color::Yellow), vec![Attribute::Bold]);

        let client = ApiClient::new();
        // get all mod info
        file_data.mods = client.fetch_all_mods().await?.mods;
        file_data.last_sync = get_current_time();

        debug!("file_data {:?}", file_data);

        info!("{} {}", "Attempting to write Mod Search file to".yellow(), search_file.display());

        let json = prettify(&file_data, "Mods Search DB")?;
        write_json_file(&search_file, json, &Config::get_path()).await?;
        
        info!("{}", "Mods Search Sync file written successfully".green());
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
        match parse_json_file::<GameVersionSync>(&file).await {
            Ok(json) => json,
            Err(e) => {
                error!("Game version sync file parse error: {}", e);
                // delete the file and recreate it
                tokio::fs::remove_file(&file).await?;
                GameVersionSync::new()
            }
        }
    } else {
        GameVersionSync::new()
    };
    
    let sync_time = config.sync_latest_game_version_file_every;
    
    if file_data.game_versions.is_empty() || force || timestamp_older_than(sync_time, &file_data.last_sync){
        notice("Syncing latest game versions..", Some(Color::Yellow), vec![Attribute::Bold]);
        
        let client = ApiClient::new();
        let gvs = client.fetch_game_versions().await?;
        file_data.game_versions = gvs.into_iter().collect();
        file_data.last_sync = get_current_time();
        
        let json = prettify(&file_data, "Game Version Sync")?;
        
        write_json_file(&file, json, &Config::get_path()).await?;

        info!("{}", "Mods Search Sync file written successfully".green());
        
    }
    
    
     if config.show_execution_time && force {
        elapsed_footer(start_time, "Game Version Sync");
    } 
    
    Ok(file_data)
}




