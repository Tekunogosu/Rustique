use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use crate::utils::{api, RustiqueOptions};
use chrono::{DateTime,Utc};

#[derive(Deserialize, Serialize, Debug)]
pub struct RustiqueSyncJson {
    #[serde(rename = "RustiqueSync")]
    pub rustique_sync: HashMap<String, ModSyncInfo>,
    pub last_sync: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ModSyncInfo {
    pub file_name: String,
    pub installed_version: String,
    pub latest_known_version: String,
    pub latest_download_url: String,
}

pub const SYNC_FILE_NAME: &str = "rustique-sync.json";


fn parse_sync_file(dir: PathBuf) -> Result<RustiqueSyncJson, Box<dyn Error>> {
    let file = File::open(dir.join(SYNC_FILE_NAME))?;
    let mut file_contents = String::new();
    let json = serde_json::from_str::<RustiqueSyncJson>(&file_contents)?;

    Ok(json)
}

fn get_current_time() -> String {
    let now = SystemTime::now();
    let datetime: DateTime<Utc> = now.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

pub fn sync(rustique_opts: RustiqueOptions) -> Result<(),Box<dyn Error>> {

    // check if rustique-sync.json exists
    // if so, parse the file for updating
    // if not, do all the sync process and then write a new file

    let file_path = rustique_opts.mod_dir.join(SYNC_FILE_NAME);
    let sync_data = if file_path.exists() {
        parse_sync_file(file_path)?
    } else {
        RustiqueSyncJson {
            rustique_sync: HashMap::new(),
            last_sync: get_current_time(),
        }
    };





    Ok(())
}