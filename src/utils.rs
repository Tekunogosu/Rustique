use std::error::Error;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use dirs::home_dir;
use rayon::prelude::IntoParallelRefIterator;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;
use crate::api_structs::ModInfo;

pub const API_BASE: &str = "https://mods.vintagestory.at/api";

pub mod api {
    use super::API_BASE;

    pub fn uri(endpoint: &str) -> String {
        format!("{}/{}", API_BASE, endpoint)
    }

    pub fn mods() -> String {
        uri("mods")
    }

    pub fn get_mod(mod_id: &str) -> String {
        uri(&format!("mods/{}", mod_id))
    }
}


pub struct RustiqueOptions {
    pub mod_dir: PathBuf,
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
                mod_dir: PathBuf::from(path).join("Vintagestory").join("Mods"),
            }
        }
        panic!("Unable to determine default mods directory");
    }

    pub fn linux() -> Self {
        if let Some(home) = home_dir() {
            return RustiqueOptions {
                mod_dir: home.join(".config").join("VintagestoryData").join("Mods"),
            };
        }
        panic!("Unable to determine user's home directory, do you have permissions??");
    }
}


// if the path contains ~/, which is short for /home/<user>, then expand it, otherwise just return
// the path,
// TODO: Need handle windows default
pub fn get_expanded_path(dir: PathBuf) -> PathBuf {
    let out = PathBuf::new();
    if dir.starts_with("~/") {
        if let Some(home) = home_dir() {
            return out.join(home).join(dir.strip_prefix("~/").unwrap());
        }
    }

    dir
}

pub fn _get_case_insensitive<'a>(obj: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    if let Some(obj) = obj.as_object() {
        obj.iter()
            .find(|(k, _)|k.to_lowercase() == key.to_lowercase())
            .map(|(_, v)| v)
    } else {
        None
    }
}

fn box_error(error: String) -> Box<dyn Error> {
    Box::new(std::io::Error::new(std::io::ErrorKind::Other, error))
}

pub fn extract_zip_metadata(entry: PathBuf) -> Result<ModInfo, Box<dyn Error>> {

    if entry.is_dir() {
        return Err(box_error(format!("Skipping mods that are not zip archives: {}", entry.display())));
    }

    if entry.extension().map_or(false, |x| x.to_ascii_lowercase() != "zip") {
        return Err(box_error(format!("Skipping non-zip file: {}", entry.display())));
    }

    let file  = File::open(&entry)
        .map_err(|e| format!("Failed to open {:?}: {}", entry.file_name(), e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to open zip archive {:?}: {}", entry.file_name(),e))?;

    let mut mod_info_file = archive.by_name("modinfo.json")
        .map_err(|e| format!("Failed to find modinfo.json in {:?}: {}", entry.file_name(),e))?;

    let mut mod_info_contents = String::new();
    mod_info_file.read_to_string(&mut mod_info_contents)
        .map_err(|e| format!("Failed to read modinfo.json in {:?}: {}", entry.file_name(),e))?;

    let mod_info = serde_json5::from_str::<ModInfo>(&mod_info_contents)
        .map_err(|e|format!("Failed to parse json in {:?}: {}", entry.file_name(),e))?;

    Ok(mod_info)
}

pub fn extract_all_mods_metadata(mod_dir: RustiqueOptions) -> Result<ModInfo, Box<dyn Error>> {
    // TODO: check which platform we are on
    let dir = fs::read_dir(mod_dir.mod_dir)?;
    let mut entries_vec: Vec<DirEntry> = dir.filter_map(|e| e.ok()).collect();

    entries_vec.sort_by(|a, b| {
        let a_name = a.file_name().to_string_lossy().to_lowercase();
        let b_name = b.file_name().to_string_lossy().to_lowercase();
        a_name.cmp(&b_name)
    });

    let mods = Arc::new(Mutex::new(Vec::<ModInfo>::new()));

    entries_vec.par_iter().for_each(|entry| {
        // println!("{:?}", entry.path());
        // we use a closure here to manage the
        match (|| -> Result<ModInfo, Box<dyn Error>> {

            extract_zip_metadata(entry.path())

        })() {
            Ok(mod_info) => mods.lock().unwrap().push(mod_info),
            Err(e) => println!("Error processing mod: {}", e),
        }
    });

    Ok(mods.lock().unwrap().clone())
}