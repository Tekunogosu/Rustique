use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use crate::utils::{RustiqueOptions, get_current_time, extract_all_mods_metadata};
use crate::api::api::ApiClient;
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use serde_json::to_string_pretty;
use crate::api_structs::{Mod, ModInfo};
use ureq::Agent;

#[derive(Deserialize, Serialize, Debug)]
pub struct RustiqueSyncJson {
    #[serde(rename = "RustiqueSync")]
    pub rustique_sync: HashMap<String, ModSyncInfo>,
    pub last_sync: String,
}

impl RustiqueSyncJson {
    pub fn new() -> RustiqueSyncJson {
        Self {
            rustique_sync: HashMap::<String, ModSyncInfo>::new(),
            last_sync: String::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ModSyncInfo {
    pub file_name: String,
    pub installed_version: String,
    pub latest_known_version: String,
    pub latest_download_url: String,
}

pub const SYNC_FILE_NAME: &str = "rustique-sync.json";


pub fn parse_sync_file(dir: PathBuf) -> Result<RustiqueSyncJson, Box<dyn Error>> {
    let mut file = File::open(dir.join(SYNC_FILE_NAME))?;
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)?;
    let json = serde_json::from_str::<RustiqueSyncJson>(&file_contents)?;

    Ok(json)
}

pub fn sync(rustique_opts: RustiqueOptions) -> Result<(),Box<dyn Error>> {

    // check if rustique-sync.json exists
    // if so, parse the file for updating
    // if not, do all the sync process and then write a new file

    let file_path = rustique_opts.mod_dir.as_ref().unwrap().join(SYNC_FILE_NAME);

    println!("rustique-sync.json: {}", file_path.display());

    let sync_data =
        RustiqueSyncJson {
            rustique_sync: HashMap::new(),
            last_sync: get_current_time(),
        };

    // wrap the sync_data in an arc/mutex for our threads
    // mut isn't required as Mutex defines that internally
    let sync_data = Arc::new(Mutex::new(sync_data));

    let installed_mods= extract_all_mods_metadata(rustique_opts)
        .map_err(|e| e.to_string())?;

    installed_mods.iter().for_each(|(k,v)| {
       sync_data.lock().unwrap()
           .rustique_sync
           .entry(v.mod_id.clone())
           .or_insert_with(|| ModSyncInfo {
               installed_version: v.version.clone().unwrap_or(String::new()),
               file_name: k.clone(),
               latest_download_url: String::new(),
               latest_known_version: String::new(),
           });
    });

    let result: HashMap<String, Mod> = ApiClient::new()
        .fetch_mods_parallel(installed_mods.into_values().collect::<Vec<ModInfo>>())?;

    result.iter().for_each(|(mod_id, mod_info)| {

        let latest_known_version = &mod_info.mod_json.releases[0].mod_version;
        let latest_download_url = &mod_info.mod_json.releases[0].main_file;

        sync_data.lock().unwrap()
            .rustique_sync
            .entry(mod_id.clone())
            .and_modify(|sync_info| {
                sync_info.latest_known_version = latest_known_version.clone().unwrap_or(String::new());
                sync_info.latest_download_url = latest_download_url.clone().unwrap_or(String::new());
            })
            .or_insert_with(|| ModSyncInfo {
                latest_known_version: latest_known_version.clone().unwrap_or(String::new()),
                latest_download_url: latest_download_url.clone().unwrap_or(String::new()),
                file_name: "None".to_string(),
                installed_version: "None".to_string(),
            });
    });

    let data = sync_data.lock().unwrap();
    let json = to_string_pretty(&*data)?;
    let mut file = File::create(file_path)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

