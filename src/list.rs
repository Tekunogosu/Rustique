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
use crate::utils::{extract_zip_metadata, RustiqueOptions};


// TODO:: Should we handle mods that are in directories and not .zip files
pub fn list_installed(mod_dir: RustiqueOptions) -> Result<Vec<ModInfo>, Box<dyn Error>> {
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
