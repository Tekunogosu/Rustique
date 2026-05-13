use std::default::Default;
use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::debug;
use crate::aliases::{ModFileName, ModID, ModName, ModVersion};
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::PathRef;
use crate::utils::{get_current_time, prettify};

#[derive(Deserialize, Serialize, Debug)]
pub struct RustiqueSyncJson {
    #[serde(rename = "RustiqueSync")]
    pub rustique_sync: BTreeMap<ModID, ModSyncInfo>,
    pub last_sync: String,
}

impl Default for RustiqueSyncJson {
    fn default() -> Self {
        RustiqueSyncJson {
            // converted from hashmap to btree to maintain sorted values. 
            // *technically* btreemap is slower, but its so not noticeable, at least in testing.
            rustique_sync: BTreeMap::default(), 
            last_sync: get_current_time()
        }
    }
}

impl RustiqueSyncJson {

    // Let the calling function tell us where the sync file is located
    pub async fn save(&self, file_location: impl PathRef) -> Result<(), RustiqueError> {
       
        debug!("Attempting to save {:?}", self);
        
        let json = prettify(self, "Sync")?;
        
        // Use tokio's async file operations
        let mut file = File::create(&file_location)
            .await
            .map_err(|e| RustiqueError::IoError {
                context: format!("Error writing sync file to {}", file_location.as_ref().to_string_lossy()),
                source: e,
            })?;

        AsyncWriteExt::write_all(&mut file, json.as_bytes()).await?;

        Ok(())
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
    pub asset_id: i64,
    pub installed_version: ModVersion,
    pub latest_known_version: ModVersion,
    pub latest_download_url: String,
    pub game_versions: Vec<String>,
    pub latest_changelog: String,
    
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