

use std::error::Error;
use std::fmt::format;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::{stdin, Read};
use std::sync::{Arc, Mutex};
use rayon::prelude::*;
use ureq::get;
use zip::ZipArchive;
use crate::api_structs::ModInfo;
use crate::utils::{get_case_insensitive, ModOptions};


// TODO:: Should we handle mods that are in directories and not .zip files
pub fn list_installed(mod_dir: ModOptions) -> Result<Vec<ModInfo>, Box<dyn Error>> {
    // TODO: check which platform we are on
    let dir = fs::read_dir(mod_dir.moddir)?;
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
        match (|| -> Result<ModInfo, String> {
            let path = entry.path();

            if path.is_dir() {
                return Err(format!("Skipping mods that are not zip archives: {}", path.display()));
            }

            if path.extension().map_or(false, |x| x.to_ascii_lowercase() != "zip") {
                return Err(format!("Skipping non-zip file: {}", path.display()));
            }

            let file  = File::open(path)
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
        })() {
            Ok(mod_info) => mods.lock().unwrap().push(mod_info),
            Err(e) => println!("Error processing mod: {}", e),
        }
    });

    Ok(mods.lock().unwrap().clone())
}

// maybes**
pub fn list_outdated() {}

pub fn list_updated() {}

// end maybes
