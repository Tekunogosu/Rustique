use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::{fs, io};
use std::fs::{DirEntry, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use chrono::{DateTime, Utc};
use colored::Colorize;
use dirs::home_dir;
use rayon::prelude::*;
use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;
use zip::result::ZipError;
use zip::ZipArchive;
use crate::aliases::{ModFileName, ModID, ModVersion};
use crate::api::ApiClient;
use crate::api_structs::ModInfo;
use crate::rustique_errors::RustiqueError;

#[derive(Clone, Debug)]
pub struct RustiqueOptions {
    pub mod_dir: Option<PathBuf>,
    pub mod_id: Option<String>,
}

impl RustiqueOptions {
    pub fn default() -> Self {
        if cfg!(target_os = "windows") {
            Self::windows()
        } else {
            Self::linux()
        }
    }

    pub fn windows() -> Self {
        if let Some(path) = std::env::var_os("APPDATA") {
            return RustiqueOptions {
                mod_dir: Some(PathBuf::from(path).join("Vintagestory").join("Mods")),
                mod_id: None,
            }
        }
        panic!("Unable to determine default mods directory");
    }

    pub fn linux() -> Self {
        if let Some(home) = home_dir() {
            return RustiqueOptions {
                mod_dir: Some(home.join(".config").join("VintagestoryData").join("Mods")),
                mod_id: None,
            };
        }
        panic!("Unable to determine user's home directory, do you have permissions??");
    }
}

pub fn get_current_time() -> String {
    let now = SystemTime::now();
    let datetime: DateTime<Utc> = now.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

// if the path contains ~/, which is short for /home/<user>, then expand it, otherwise just return
// the path,
// TODO: Need handle windows default
pub fn get_expanded_path(dir: PathBuf) -> PathBuf {
    if dir.starts_with("~/") {
        if let Some(home) = home_dir() {
            return PathBuf::new().join(home).join(dir.strip_prefix("~/").unwrap());
        }
    }

    dir
}

// this function filters out any unwanted dependencies
pub fn find_missing_dependencies(
    dependencies: Option<HashMap<ModID, ModVersion>>,
    excluded_ids: Option<&HashSet<ModID>>,
) -> Vec<ModID> {
    let default_exclusions = ["game", "survival", "creative"];
    let empty_set :HashSet<ModID> = HashSet::new();
    let excluded = excluded_ids.unwrap_or(&empty_set);
    dependencies.unwrap_or_default()
        .keys()
        .filter(|mod_id|
            !default_exclusions.contains(&mod_id.to_lowercase().as_str())
            && !excluded.contains(&mod_id.to_lowercase().to_string())
        ).cloned().collect()
}


#[cfg(feature = "debug")]
pub fn dlog(msg: &str) {
    println!("DEBUG: {}", msg);
}

#[cfg(not(feature = "debug"))]
pub fn dlog(_msg: &str) {}

pub fn extract_zip_metadata(entry: PathBuf) -> Result<ModInfo, RustiqueError> {
    if entry.is_dir() {
        return Err(RustiqueError::ModNotZipped(entry.display().to_string()));
    }

    if entry.extension().map_or(false, |x| x.to_ascii_lowercase() != "zip") {
        return Err(RustiqueError::SimpleError(format!("Skipping non-zip file: {}", entry.display())));
    }

    let file  = File::open(&entry)
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

pub fn extract_all_mods_metadata(mod_dir: &PathBuf) -> Result<HashMap<ModFileName, ModInfo>, RustiqueError> {

    let dir = fs::read_dir(mod_dir)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Can't read mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;

    let entries_vec: Vec<DirEntry> = dir.filter_map(|e| e.ok()).collect();

    let mods = Arc::new(Mutex::new(HashMap::<ModFileName, ModInfo>::new()));

    entries_vec.par_iter().for_each(|entry| {
        let filename = entry.file_name().to_string_lossy().to_string();
        match (|| -> Result<ModInfo, RustiqueError> {
            extract_zip_metadata(entry.path())
        })() {
            Ok(mod_info) => {mods.lock().unwrap().insert(filename, mod_info);}
            Err(e) =>  {
                if matches!(e, RustiqueError::ModNotZipped(_)) {
                    eprintln!("{}",e.to_string().red().bold());
                } else {
                    dlog(&format!("{}", e.to_string()));
                }
            }
        }
    });

    Ok(mods.lock().unwrap().clone())
}

pub fn delete_file(file: &Path) -> Result<(), RustiqueError> {
    dlog(format!("Trying to delete {}", file.display()).as_str());
    if file.exists() {
        Ok(fs::remove_file(&file)
            .map_err(|e| RustiqueError::IoError {
                context: format!("Failed attempting to delete {}", file.file_name().unwrap().to_string_lossy()),
                source: e,
            })?)
    } else {
        Err(RustiqueError::SimpleError(format!("File {} no longer exists..", file.display())))
    }
}

pub fn download_mod(mod_dir: &PathBuf, download_url: &String, api_client: &ApiClient) -> Result<ModInfo, RustiqueError> {

    let filename_before = &download_url.split('=').last().unwrap();
    let file_path_before = PathBuf::from(mod_dir.clone().join(filename_before));

    // Replace any spaces in the downloaded file with _ . This makes it easier to process later
    let filename_fix = mod_dir.clone().join(filename_before).to_string_lossy().replace(" ", "_");
    let file_path = PathBuf::from(filename_fix);

    if file_path.exists() && file_path_before.exists() {
        return Err(RustiqueError::SimpleError(format!("File {} already exists.", file_path.display())))
    }

    let url = Url::parse(download_url.as_str())
        .map_err(|e| RustiqueError::UrlParseError(e))?;

    dlog(format!("Trying to download url: {}", url.clone().to_string()).as_str());
    let response = api_client.get_request(&url.to_string())
        .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;

    let mut bytes: Vec<u8> = Vec::new();

    response.into_body().into_reader().read_to_end(&mut bytes)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failure reading response from API {}", download_url.red()),
            source: e,
        })?;

    let mut file = File::create(&file_path)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Unable to create file {}", file_path.to_string_lossy()),
            source: e
        })?;

    file.write_all(&bytes)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failure while writing to byte array for {}", file_path.to_string_lossy()),
            source: e
        })?;

    dlog(format!("File downloaded to {}", file_path.display()).as_str());

    Ok(extract_zip_metadata(file_path)?)
}

