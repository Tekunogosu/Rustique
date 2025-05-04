use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use colored::Colorize;
use rayon::prelude::*;
use serde_json::to_string_pretty;
use ureq::Agent;
use semver::{Error, Version};
use tracing::{debug, error, info, warn};
use crate::aliases::{ModFileName, ModID, ModVersion};
use crate::rustique_errors::RustiqueError;
use crate::api_structs::{Mod, ModInfo, Releases};
use crate::utils::{RustiqueOptions, get_current_time, extract_all_mods_metadata, dlog, footer};
use crate::api::ApiClient;
use crate::rustique_errors::RustiqueError::UrlParseError;
use crate::version_management::{parse_latest_version, parse_version};

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
            last_sync: String::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModSyncInfo {
    pub file_name: ModFileName,
    pub installed_version: ModVersion,
    pub latest_known_version: ModVersion,
    pub latest_download_url: String,
}


pub fn handle_sync_call(mod_dir: &PathBuf) {
    match sync(mod_dir) {
        Ok(_) => {}
        Err(e) => {
           error!("{}", e.to_string());
            exit(1);
        }
    }
}

pub const SYNC_FILE_NAME: &str = "rustique-sync.json";

pub fn parse_sync_file(mod_dir: &PathBuf) -> Result<RustiqueSyncJson, RustiqueError> {
    let mut file = File::open(mod_dir.join(SYNC_FILE_NAME)).map_err(|e| RustiqueError::IoError {
        context: format!("Unable to open {}", SYNC_FILE_NAME),
        source: e,
    })?;

    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents).map_err(|e| RustiqueError::IoError {
        context: format!("Failure while reading from file {}", SYNC_FILE_NAME),
        source: e
    })?;

    let json = serde_json5::from_str::<RustiqueSyncJson>(&file_contents)
        .map_err(|e| RustiqueError::JsonError {
            context: format!("Json parsing Error for {}", SYNC_FILE_NAME),
            source: e
        })?;

    Ok(json)
}

pub fn sync(mod_dir: &PathBuf) -> Result<(), RustiqueError> {
    eprintln!("{}", "Syncing...".green().bold());
    let start_time = Instant::now();
    // check if rustique-sync.json exists
    // if so, parse the file for updating
    // if not, do all the sync process and then write a new file

    let file_path = mod_dir.join(SYNC_FILE_NAME);

    debug!("sync file: {}", file_path.display());

    let sync_data =
        RustiqueSyncJson {
            rustique_sync: HashMap::new(),
            last_sync: get_current_time(),
        };

    // wrap the sync_data in an arc/mutex for our threads
    // mut isn't required as Mutex defines that internally
    let sync_data = Arc::new(Mutex::new(sync_data));

    let installed_mods= extract_all_mods_metadata(mod_dir)?;

    installed_mods.iter().for_each(|(k,v)| {
        let version = if let Ok(parsed_version) = parse_version(v.version.clone().unwrap_or_default()) {
            parsed_version.to_string()
        } else {
            warn!("Could not parse version: {} for {}\n\rThis mod may not update correctly..", v.version.clone().unwrap_or_default(), k.to_string());
            v.version.clone().unwrap_or_default()
        };

        info!("VERSION Parsed: {} for {}", version, v.mod_id);

       sync_data.lock().unwrap()
           .rustique_sync
           .entry(v.mod_id.clone())
           .or_insert_with(|| ModSyncInfo {
               installed_version: version.clone(),
               file_name: k.clone(),
               latest_download_url: String::new(),
               latest_known_version: String::new(),
           });

    });

    let result: HashMap<ModID, Mod> = ApiClient::new()
        .fetch_mods_parallel(installed_mods.into_values().collect::<Vec<ModInfo>>())?;

    result.par_iter().for_each(|(mod_id, mod_info): (&ModID, &Mod)| {

        let (mod_version, download_url) = parse_latest_version(&mod_info.mod_json.releases);

        sync_data.lock().unwrap()
            .rustique_sync
            .entry(mod_id.clone())
            .and_modify(|sync_info| {
                sync_info.latest_known_version = mod_version.clone();
                sync_info.latest_download_url = download_url.clone();
            })
            .or_insert_with(|| ModSyncInfo {
                latest_known_version: mod_version,
                latest_download_url: download_url,
                file_name: "None".to_string(),
                installed_version: "None".to_string(),
            });

    });

    // do something with the parse errors

    let data = sync_data.lock().unwrap();
    let json = to_string_pretty(&*data).map_err(|e| RustiqueError::JsonError {
        context: "Failure while making the sync json pretty".to_string(),
        source: serde_json5::Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)),
    })?;
    let mut file = File::create(file_path)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Error writing sync file to mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;

    file.write_all(json.as_bytes())?;

    // let elapsed = format!("{:.2}", start_time.elapsed().as_secs_f64());
    // println!("\n\r{} {}{}\n\r", "Sync operation took:".bright_green().bold().on_black(), elapsed.bright_purple().on_black(), "s".bright_yellow().on_black());
    //
    footer(start_time, "Sync");

    Ok(())
}

