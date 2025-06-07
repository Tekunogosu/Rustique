use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::debug;
use crate::aliases::{ModFileName, ModID, ModName, ModVersion};
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::PathRef;
use crate::utils::{get_current_time, prettify_json};

#[derive(Deserialize, Serialize, Debug)]
pub struct RustiqueSyncJson {
    #[serde(rename = "RustiqueSync")]
    pub rustique_sync: HashMap<ModID, ModSyncInfo>,
    pub last_sync: String,
    
    pub file_location: PathBuf,
}


impl RustiqueSyncJson {
    pub fn new(file_path: impl PathRef) -> RustiqueSyncJson {
        Self {
            rustique_sync: HashMap::<ModID, ModSyncInfo>::new(),
            last_sync: get_current_time(),
            file_location: file_path.as_ref().to_path_buf(),
        }
    }
    
    pub async fn save(&self) -> Result<(), RustiqueError> {
       
        debug!("Attempting to save {:?}", self);
       
        let json = prettify_json(self, "Sync")?;

        // Use tokio's async file operations
        let mut file = File::create(&self.file_location)
            .await
            .map_err(|e| RustiqueError::IoError {
                context: format!("Error writing sync file to {}", &self.file_location.to_string_lossy()),
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
