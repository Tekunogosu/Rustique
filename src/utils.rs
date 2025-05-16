use crate::aliases::{ModFileName, ModID};
use crate::api::api_structs::ModInfo;
use crate::commands::sync::{ModSyncInfo, SYNC_FILE_NAME};
use crate::config_manager::{get_config};
use crate::install_manager::Install;
use crate::rustique_errors::RustiqueError;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use owo_colors::OwoColorize;
use dirs::home_dir;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{DirEntry, File};
use std::io::{Read};
use std::path::{Path, PathBuf};
use std::{fs};
use tokio::io::AsyncWriteExt;
use tracing::{debug, error};
use zip::ZipArchive;

#[derive(Clone, Debug)]
pub struct RustiqueOptions {
    pub mod_dir: Option<PathBuf>,
}

impl RustiqueOptions {
    pub fn default() -> Self {
        if cfg!(target_os = "windows") {
            Self::windows()
        } else {
            Self::unix()
        }
    }

    pub fn windows() -> Self {
        if let Some(path) = std::env::var_os("APPDATA") {
            return RustiqueOptions {
                mod_dir: Some(PathBuf::from(path).join("Vintagestory").join("Mods")),
            }
        }
        panic!("Unable to determine default mods directory");
    }

    // this also works for mac
    pub fn unix() -> Self {
        if let Some(home) = home_dir() {
            return RustiqueOptions {
                mod_dir: Some(home.join(".config").join("VintagestoryData").join("Mods")),
            };
        }
        panic!("Unable to determine user's home directory, do you have permissions??");
    }

    pub async fn get_mod_path(&self) -> PathBuf {
        let default_path = Self::default().mod_dir.unwrap();
        let config = get_config().read().await;
        let config_mod_dir = PathBuf::from(&config.mod_dir);

        if default_path.as_path().eq(get_expanded_path(config_mod_dir.clone()).as_path()) {
            default_path
        } else {
            config_mod_dir
        }
    }
}

pub fn get_current_time() -> String {
    let datetime: DateTime<Utc> = Utc::now();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

pub fn timestamp_older_than(num_hours: i64, timestamp: &str) -> bool {

    let naive_dt = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M").map_err(|e| {error!("{}", e)}).unwrap_or_default();
    let now = Utc::now().naive_utc();
    let duration = now.signed_duration_since(naive_dt);

    duration > Duration::hours(num_hours)
}

// if the path contains ~/, which is short for /home/<user>, then expand it, otherwise just return
// the path,
// TODO: Need handle windows default
pub fn get_expanded_path(dir: PathBuf) -> PathBuf {
    if dir.starts_with("~/") {
        if let Some(home) = home_dir() {
            let d = match dir.strip_prefix("~") {
                Ok(d) => d,
                Err(e) => panic!("{}", e),
            };
            return PathBuf::new().join(home).join(d);
        }
    }

    dir
}

pub fn extract_zip_metadata(entry: &PathBuf) -> Result<ModInfo, RustiqueError> {
    // This function doesn't need async as it's doing synchronous file operations
    if entry.is_dir() {
        return Err(RustiqueError::ModNotZipped(entry.display().to_string()));
    }
    if entry.extension().is_some_and(|x| !x.eq_ignore_ascii_case("zip")) {
        return Err(RustiqueError::SimpleError(format!("Skipping non-zip file: {}", entry.display())));
    }
    let file = File::open(entry)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to open {:?}: {}", entry.file_name(), e),
            source: e,
        })?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Failed to open zip archive {:?}: {}", entry.file_name(),e),
            source: e
        })?;
    let mut mod_info_file = archive.by_name("modinfo.json")
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Failed to find modinfo.json in {:?}: {}", entry.file_name(),e),
            source: e
        })?;
    let mut mod_info_contents = String::new();
    mod_info_file.read_to_string(&mut mod_info_contents)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to read modinfo.json in {:?}", entry.file_name()),
            source: e,
        })?;
    let mod_info = serde_json5::from_str::<ModInfo>(&mod_info_contents)
        .map_err(|e: serde_json5::Error| RustiqueError::JsonError {
            context: format!("Failed to parse json in {}", entry.file_name().unwrap_or_default().to_string_lossy()),
            source: e
        })?;
    Ok(mod_info)
}

pub async fn extract_all_mods_metadata(mod_dir: &PathBuf) -> Result<HashMap<ModFileName, ModInfo>, RustiqueError> {

    let dir = fs::read_dir(mod_dir)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Can't read mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;
    let entries_vec: Vec<DirEntry> = dir.filter_map(|e| e.ok()).collect();
    // let mods = Arc::new(Mutex::new(HashMap::<ModFileName, ModInfo>::new()));

    let config = get_config().read().await;
    
    // Use Rayon for CPU-bound tasks (zip processing is CPU-bound)
    let results:Vec<(ModFileName, ModInfo)> = entries_vec.par_iter()
        .filter_map(|entry| {
            let filename = entry.file_name().to_string_lossy().to_string();
            match extract_zip_metadata(&entry.path()) {
                Ok(mod_info) => Some((filename, mod_info)),
                Err(e) => {
                     if matches!(e, RustiqueError::ModNotZipped(_)) && config.notify_of_unzipped_mods {
                        println!("{}",e.to_string().yellow());
                    } else {
                        debug!("{}", e.to_string().yellow());
                    }
                    None
                }
            }
        }).collect();

      Ok(results.into_iter().collect())
}

pub fn verify_zip_file(file_path: &PathBuf) -> Result<(), RustiqueError> {
    // Open and verify the zip file integrity
    let file = File::open(file_path)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to open file for verification: {}", file_path.to_string_lossy()),
            source: e,
        })?;

    let archive = ZipArchive::new(file)
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Invalid zip file: {}", file_path.to_string_lossy()),
            source: e
        })?;

    // Check that the archive contains at least one file
    if archive.is_empty() {
        return Err(RustiqueError::SimpleError(format!("Zip file is empty: {}", file_path.to_string_lossy())));
    }

    Ok(())
}

pub async fn delete_file(file: &Path) -> Result<(), RustiqueError> {
    debug!("Trying to delete {}", file.display());
    if file.exists() && !file.is_dir() {
        tokio::fs::remove_file(file).await
            .map_err(|e| RustiqueError::IoError {
                context: format!("Failed attempting to delete {}", file.file_name().unwrap().to_string_lossy()),
                source: e,
            })
    } else {
        Err(RustiqueError::SimpleError(format!("File {} is no longer there!", file.display())))
    }
}

// Replaces all instances of the newline and tab character from text, as well as excessive spaces.
// This is a fix for https://github.com/Tekunogosu/Rustique/issues/3
pub fn sanitize_string(string: &str) -> String {
    // let re = Regex::new(r"[\n\t ]+").unwrap();
    // re.replace_all(string, " ").to_string()
    string
        .split_whitespace()
        .fold(String::new(), |mut acc, word| {
            if !acc.is_empty() {
                acc.push(' ');
            }
            acc.push_str(word);
            acc
        })
}

// Helper function to get just installed dependencies by passing empty vec and hashmap to the parts that filter out dependencies
pub fn gather_dependencies(installed_mods: &HashMap<ModFileName, ModInfo>) -> Vec<Install> {
    gather_missing_dependencies(installed_mods, &[], &HashMap::new())
}

pub fn gather_missing_dependencies(installed_mods: &HashMap<ModFileName, ModInfo>, mods_requested: &[ModID], sync_data: &HashMap<ModID, ModSyncInfo>) -> Vec<Install> {
    // if there are reports of slowness is this section .values().par_bridge()...flat_map_iter() could be used to speed it up
    // this is prob not an issue even with a lot of mods as the data is all in memory at this point
    let id_vec: Vec<ModID> = sync_data.keys().cloned().collect();

    installed_mods
        .values()
        .filter(|mod_info| mods_requested.is_empty() || mods_requested.contains(&mod_info.mod_id))
        .flat_map(|mod_info| {
            mod_info.dependencies.as_ref()
                .map(|hm| hm.iter()
                    .filter_map(|(mod_id, version)|
                        if !mod_id.contains("game")
                            && !mod_id.contains("survival")
                            && !mod_id.contains("creative")
                            && !id_vec.contains(mod_id) {
                            Some(Install {
                                mod_id: mod_id.clone(),
                                mod_name: String::new(),
                                version_to_install: version.clone(),
                                download_url: String::new(),
                                current_file_path: None,
                            })
                        } else {
                            None
                        }).collect::<Vec<_>>()
                ).unwrap_or_default()
                .into_iter()
        }).collect()
}


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
        .map_err(|e| {
            let sync_error = if file_path.file_name().unwrap().to_string_lossy().eq(SYNC_FILE_NAME) {
                format!("{} {} {}", "(Run".yellow(), "Rustique sync".blue(), "to repopulate the sync file and resolve this message)".yellow())
            } else {
                String::new()
            };
            RustiqueError::JsonError {
                context: format!("Json parsing Error for {filename} {sync_error}"),
                source: e
            }
        })?;

    Ok(json)
}

pub async fn write_json_file(file_path: &PathBuf, json: String, config_dir: &PathBuf) -> Result<(), RustiqueError> {
    let mut open_file = tokio::fs::File::create(file_path).await.map_err(|e|
        RustiqueError::IoError {
            context: format!("Error writing sync mod search file to config dir: {}", config_dir.to_string_lossy()),
            source: e,
        }
    )?;
    AsyncWriteExt::write_all(&mut open_file, json.as_bytes()).await?; 
    
    Ok(())
}

