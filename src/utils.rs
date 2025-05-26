use crate::config::config_manager::Config;
use crate::aliases::{ModFileName, ModID};
use crate::api::api_structs::{ModApi, ModInfo};
use crate::commands::sync::{GameVersionSync, ModSyncInfo};
use crate::install_manager::{Install, Installed};
use crate::rustique_errors::RustiqueError;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use owo_colors::OwoColorize;
use dirs::home_dir;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{DirEntry, File};
use std::io::{Read};
use std::path::PathBuf;
use std::fs;
use std::process::exit;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};
use zip::ZipArchive;
use crate::config::config_manager::get_config;
use crate::consts::{FILE_GAME_VERSION_SYNC, FILE_MODINFO_JSON, FILE_RUSTIQUE_SYNC};
use crate::modpack::symlink_manager::SymlinkManager;
use crate::traits::ref_ext::PathRef;
use crate::traits::string_ext::StrLowerExt;

#[derive(Clone, Debug)]
pub struct RustiqueOptions {
    pub mod_dir: Option<PathBuf>,
}

impl RustiqueOptions {
    pub fn default() -> Self {
        #[cfg(windows)]
        return Self::windows();
        
        #[cfg(unix)]
        return Self::unix();
    }

    #[cfg(windows)]
    pub fn windows() -> Self {
        if let Some(path) = std::env::var_os("APPDATA") {
            return RustiqueOptions {
                mod_dir: Some(PathBuf::from(path).join("Vintagestory").join("Mods")),
            }
        }
        panic!("Unable to determine default mods directory");
    }

    // this also works for mac
    #[cfg(unix)]
    pub fn unix() -> Self {
        // TODO: check if dir exists, if not check for the flatpack dir, throw error message if none are found
        if let Some(home) = home_dir() {
            let base =  home
                .join(".config")
                .join("VintagestoryData")
                .join("Mods");
            
            let flatpak = home
                .join(".var")
                .join("app")
                .join("at.vintagestory.VintageStory")
                .join("config")
                .join("VintagestoryData")
                .join("Mods");
            
            let mut options = RustiqueOptions {
                mod_dir: Some(PathBuf::new())
            };
            
            if base.exists() {
                info!("normal mod dir found");
                options.mod_dir = Some(base);
            } else if flatpak.exists() {
                info!("flatpak mod dir found");
                options.mod_dir = Some(flatpak);
            } else {
                info!("Rustique was unable to find the default mod dir. Using empty dir for now.");
                options.mod_dir = None;
            }
            
            return options
        }
        panic!("Unable to determine user's home directory, do you have permissions??");
    }

    pub async fn get_mod_path(&self) -> PathBuf {
        let default_path = self.mod_dir.clone().unwrap_or_default();
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
pub fn get_expanded_path(dir: impl PathRef) -> PathBuf {
    let dir = dir.as_ref();
    if dir.starts_with("~/") {
        if let Some(home) = home_dir() {
            let d = match dir.strip_prefix("~") {
                Ok(d) => d,
                Err(e) => panic!("{}", e),
            };
            return PathBuf::new().join(home).join(d);
        }
    }

    dir.to_path_buf()
}

pub fn extract_zip_metadata<T>(entry: impl PathRef, inner_file: &str) -> Result<T, RustiqueError>
where T: for<'de> serde::Deserialize<'de>
{
    let entry = entry.as_ref(); 
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
    let mut mod_info_file = archive.by_name(inner_file)
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Failed to find {} in {:?}: {}", inner_file, entry.file_name(), e),
            source: e
        })?;
    let mut mod_info_contents = String::new();
    mod_info_file.read_to_string(&mut mod_info_contents)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to read {} in {:?}", inner_file, entry.file_name()),
            source: e,
        })?;
    let mod_info = if inner_file.to_lowercase().ends_with(".json") {
        serde_json5::from_str::<T>(&mod_info_contents)
            .map_err(|e: serde_json5::Error| RustiqueError::JsonError {
                context: format!("Failed to parse json in {}", entry.file_name().unwrap_or_default().to_string_lossy()),
                source: e
            })?
    } else if inner_file.to_lowercase().ends_with(".toml") {
        toml::from_str::<T>(&mod_info_contents)
            .map_err(|e| RustiqueError::TomlError {
                context: format!("Failed to parse toml in {}", entry.file_name().unwrap_or_default().to_string_lossy()),
                source: e
            })?
    } else {
        return Err(RustiqueError::SimpleError(format!("Unsupported file format {inner_file}")))
    };
        
    Ok(mod_info)
}

/// 
pub async fn extract_all_mods_metadata(mod_dir: impl PathRef, ignore_symlink: bool) -> Result<HashMap<ModFileName, ModInfo>, RustiqueError> {
    let mod_dir = mod_dir.as_ref();
    let dir = fs::read_dir(mod_dir)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Can't read mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;
    let entries_vec: Vec<DirEntry> = dir.filter_map(|e| e.ok()).collect();
    // let mods = Arc::new(Mutex::new(HashMap::<ModFileName, ModInfo>::new()));

    let config = get_config().read().await;
    
    // Use Rayon for CPU-bound tasks (zip processing is CPU-bound)
    let results:Vec<(ModFileName, ModInfo)> = entries_vec
        .par_iter()
        // This is to ignore modpack mods when using normal rustique commands while a modpack is enabled
        .filter(|e| !(ignore_symlink && SymlinkManager::exists(e.path())))
        .filter_map(|entry| {
            let filename = entry.file_name().to_string_lossy().to_string();
            extract_zip_metadata::<ModInfo>(&entry.path(), FILE_MODINFO_JSON)
                .map(|mod_info| (filename, mod_info))
                .inspect_err(|e| {
                    if matches!(e, RustiqueError::ModNotZipped(_)) && config.notify_of_unzipped_mods { 
                        println!("{}",e.to_string().yellow());
                    } else {
                        debug!("{}", e.to_string().yellow());
                    } 
                }).ok()
        }).collect();

      Ok(results.into_iter().collect())
}

pub fn verify_zip_file(file_path: impl PathRef) -> Result<(), RustiqueError> {
    // Open and verify the zip file integrity
    let file_path = file_path.as_ref();
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

pub async fn delete_file(file: impl PathRef) -> Result<(), RustiqueError> {
    let file = file.as_ref();
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

pub fn gather_missing_dependencies<V: AsRef<[ModID]>>(installed_mods: &HashMap<ModFileName, ModInfo>, mods_requested: V, sync_data: &HashMap<ModID, ModSyncInfo>) -> Vec<Install> {
    // if there are reports of slowness is this section .values().par_bridge()...flat_map_iter() could be used to speed it up
    // this is prob not an issue even with a lot of mods as the data is all in memory at this point
    let id_vec: Vec<ModID> = sync_data.keys().cloned().collect();
    
    let mods_requested = mods_requested.as_ref();

    installed_mods
        // .values()
        .iter()
        .filter(|(_,mod_info)| mods_requested.is_empty() || mods_requested.contains(&mod_info.mod_id))
        .flat_map(|(mod_filename, mod_info)| {
            mod_info.dependencies.iter()
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
                            current_file_path: Some(PathBuf::from(mod_filename)),
                        })
                    } else {
                        None
                    }
                )
                .collect::<Vec<_>>()
                .into_iter()
        }).collect()
}


pub fn parse_json_file<T>(file_path: impl PathRef) -> Result<T, RustiqueError>
where
    T: for<'de> serde::Deserialize<'de>
{
    let file_path = file_path.as_ref();
    let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

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
            let sync_error = if filename.eq(FILE_RUSTIQUE_SYNC) {
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

pub async fn write_json_file(file_path: impl PathRef, json: String, config_dir: impl PathRef) -> Result<(), RustiqueError> {
    let (file_path , config_dir)= (file_path.as_ref(),config_dir.as_ref());
    let mut open_file = tokio::fs::File::create(file_path).await.map_err(|e|
        RustiqueError::IoError {
            context: format!("Error writing sync mod search file to config dir: {}", config_dir.to_string_lossy()),
            source: e,
        }
    )?;
    AsyncWriteExt::write_all(&mut open_file, json.as_bytes()).await?; 
    
    Ok(())
}


pub fn latest_stable() -> String {
   
    // Have to check if the file even exists first or we get weird behavior 
    // this function is called during the process in which clap creates the cli args,
    // if the file doesn't exist, the program exits immediately
    // this file will be created the first time sync is executed
    if !Config::get_path().join(FILE_GAME_VERSION_SYNC).exists() {
        return "0.0.0".into()
    }
    
    let version = sorted_game_versions();
   
    // filter out all the unstable version which end with -rc.xx
    let out: Vec<String> = version.iter().filter(|v| !v.lower_contains("-rc")).cloned().collect();
    
    out.first().unwrap_or(&String::new()).to_string()
}

pub fn sorted_game_versions() -> Vec<String> {
    let version_file_path = Config::get_path().join(FILE_GAME_VERSION_SYNC);

    let mut versions = if version_file_path.exists() {
        match parse_json_file::<GameVersionSync>(&version_file_path) {
            Ok(file_data) => file_data.game_versions,
            Err(e) => {
                eprintln!("Error: {e}");
                exit(1)
            }
        }
    } else {
        eprintln!("Unable to get latest game version by default, run Rustique sync and try again");
        exit(1)
    };
    
    versions.sort_by(|v1,v2| {
        let v1_p = lenient_semver::parse(v1).unwrap();
        let v2_p = lenient_semver::parse(v2).unwrap();
        v1_p.cmp(&v2_p)
    });

    versions.reverse();
    versions
}

pub fn find_mod_id<V: AsRef<[ModApi]>>(mod_name: &String, mod_filename: &ModFileName, mods_search_data: V) -> Result<String, RustiqueError> {
    let mods_search_data = mods_search_data.as_ref();
    info!("{} has an empty mod id, attempting locate mod id...", mod_filename);
    let res: Vec<ModApi> = mods_search_data.iter().filter(|mod_search| {
        match &mod_search.name {
            Some(name) => {
                mod_name.eq_ignore_ascii_case(name)
            }
            None => {
                mod_search.mod_id_strs.contains(mod_name)
            }
        }
    }).cloned().collect();

    if res.is_empty() || res.len() > 1 {
        // no mods match
        warn!("Unable to determine the mod_id for {} - {}.\n\r\t Their modinfo.json is malformed and no information provided allowed Rustique to determine it.\n\r\t \
                     Please contact the author to correct their modinfo.json file", mod_name.bright_red().bold(), mod_filename.bright_red().bold());
        Err(RustiqueError::SimpleError(format!("Unable to locate mod_id for {mod_name}")))
    } else {
        Ok(res[0].mod_id.to_string())
    } 
}

pub async fn remove_older_files(processed_install: &[Installed]) -> Result<(), RustiqueError> {
    for mod_installed in processed_install {
        if let (Some(old), Some(new)) = (&mod_installed.old_file_path, &mod_installed.installed_file_path) {
            if old == new {
                info!("Old file and new file have the same name, **NOT DELETING**");
            } else {
                info!("Cleaning up mod file for {}", old.display());
                delete_file(old).await?;
            }
        }
    }
    Ok(())
}