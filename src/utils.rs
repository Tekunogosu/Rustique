use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::{fs, io};
use std::fmt::Display;
use std::fs::{DirEntry, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};
use chrono::{DateTime, Utc};
use colored::Colorize;
use comfy_table::{Cell, Row, Table, Color, Attribute, CellAlignment, TableComponent};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_BORDERS_ONLY, UTF8_FULL, UTF8_HORIZONTAL_ONLY};
use dirs::home_dir;
use rayon::prelude::*;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use toml::value::Time;
use tracing::{debug, error, warn};
use tracing::span::Attributes;
use url::Url;
use zip::result::ZipError;
use zip::ZipArchive;
use crate::aliases::{ModFileName, ModID, ModVersion};
use crate::api::ApiClient;
use crate::api_structs::ModInfo;
use crate::config_manager::get_config;
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
            Self::unix()
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

    // this also works for mac
    pub fn unix() -> Self {
        if let Some(home) = home_dir() {
            return RustiqueOptions {
                mod_dir: Some(home.join(".config").join("VintagestoryData").join("Mods")),
                mod_id: None,
            };
        }
        panic!("Unable to determine user's home directory, do you have permissions??");
    }

    pub fn get_mod_path(&self) -> PathBuf {
        let default_path = Self::default().mod_dir.unwrap();
        let config = get_config().read().unwrap();
        let config_mod_dir = PathBuf::from(&config.mod_dir);

        if default_path.as_path().eq(get_expanded_path(config_mod_dir.clone()).as_path()) {
            default_path
        } else {
            config_mod_dir
        }
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


//
// pub fn extract_zip_metadata(entry: PathBuf) -> Result<ModInfo, RustiqueError> {
//     // This function doesn't need async as it's doing synchronous file operations
//     if entry.is_dir() {
//         return Err(RustiqueError::ModNotZipped(entry.display().to_string()));
//     }
//     if entry.extension().map_or(false, |x| x.to_ascii_lowercase() != "zip") {
//         return Err(RustiqueError::SimpleError(format!("Skipping non-zip file: {}", entry.display())));
//     }
//     let file = File::open(&entry)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Failed to open {:?}: {}", entry.file_name(), e),
//             source: e,
//         })?;
//     let mut archive = ZipArchive::new(file)
//         .map_err(|e| RustiqueError::ZipError {
//             context: format!("Failed to open zip archive {:?}: {}", entry.file_name(),e),
//             source: e
//         })?;
//     let mut mod_info_file = archive.by_name("modinfo.json")
//         .map_err(|e| RustiqueError::ZipError {
//             context: format!("Failed to find modinfo.json in {:?}: {}", entry.file_name(),e),
//             source: e
//         })?;
//     let mut mod_info_contents = String::new();
//     mod_info_file.read_to_string(&mut mod_info_contents)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Failed to read modinfo.json in {:?}", entry.file_name()),
//             source: e,
//         })?;
//     let mod_info = serde_json5::from_str::<ModInfo>(&mod_info_contents)
//         .map_err(|e: serde_json5::Error| RustiqueError::JsonError {
//             context: format!("Failed to parse json in {}", entry.file_name().unwrap_or_default().to_string_lossy()),
//             source: e
//         })?;
//     Ok(mod_info)
// }
//
// pub fn extract_all_mods_metadata(mod_dir: &PathBuf) -> Result<HashMap<ModFileName, ModInfo>, RustiqueError> {
//     // This can remain synchronous since Tokio won't help with CPU-bound tasks
//     let dir = fs::read_dir(mod_dir)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Can't read mod_dir: {}", mod_dir.to_string_lossy()),
//             source: e,
//         })?;
//     let entries_vec: Vec<DirEntry> = dir.filter_map(|e| e.ok()).collect();
//     let mods = Arc::new(Mutex::new(HashMap::<ModFileName, ModInfo>::new()));
//     let notify_of_unzipped_mods = match get_config().read() {
//         Ok(config) => config.notify_of_unzipped_mods,
//         Err(e) => {
//             error!("Config error: {}", e.to_string());
//             false
//         }
//     };
//
//     // Use Rayon for CPU-bound tasks (zip processing is CPU-bound)
//     entries_vec.par_iter().for_each(|entry| {
//         let filename = entry.file_name().to_string_lossy().to_string();
//         match (|| -> Result<ModInfo, RustiqueError> {
//             extract_zip_metadata(entry.path())
//         })() {
//             Ok(mod_info) => {mods.lock().unwrap().insert(filename, mod_info);}
//             Err(e) =>  {
//                 if matches!(e, RustiqueError::ModNotZipped(_)) && notify_of_unzipped_mods {
//                     eprintln!("{}",e.to_string().yellow());
//                 } else {
//                     debug!("{}", e.to_string().yellow());
//                 }
//             }
//         }
//     });
//     Ok(mods.lock().unwrap().clone())
// }
pub fn extract_zip_metadata(entry: PathBuf) -> Result<ModInfo, RustiqueError> {
    // This function doesn't need async as it's doing synchronous file operations
    if entry.is_dir() {
        return Err(RustiqueError::ModNotZipped(entry.display().to_string()));
    }
    if entry.extension().map_or(false, |x| x.to_ascii_lowercase() != "zip") {
        return Err(RustiqueError::SimpleError(format!("Skipping non-zip file: {}", entry.display())));
    }
    let file = File::open(&entry)
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
    // This can remain synchronous since Tokio won't help with CPU-bound tasks
    let dir = fs::read_dir(mod_dir)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Can't read mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;
    let entries_vec: Vec<DirEntry> = dir.filter_map(|e| e.ok()).collect();
    let mods = Arc::new(Mutex::new(HashMap::<ModFileName, ModInfo>::new()));
    let notify_of_unzipped_mods = match get_config().read() {
        Ok(config) => config.notify_of_unzipped_mods,
        Err(e) => {
            error!("Config error: {}", e.to_string());
            false
        }
    };

    // Use Rayon for CPU-bound tasks (zip processing is CPU-bound)
    entries_vec.par_iter().for_each(|entry| {
        let filename = entry.file_name().to_string_lossy().to_string();
        match (|| -> Result<ModInfo, RustiqueError> {
            extract_zip_metadata(entry.path())
        })() {
            Ok(mod_info) => {mods.lock().unwrap().insert(filename, mod_info);}
            Err(e) =>  {
                if matches!(e, RustiqueError::ModNotZipped(_)) && notify_of_unzipped_mods {
                    eprintln!("{}",e.to_string().yellow());
                } else {
                    debug!("{}", e.to_string().yellow());
                }
            }
        }
    });
    Ok(mods.lock().unwrap().clone())
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


// pub fn extract_zip_metadata(entry: PathBuf) -> Result<ModInfo, RustiqueError> {
//     if entry.is_dir() {
//         return Err(RustiqueError::ModNotZipped(entry.display().to_string()));
//     }
//
//     if entry.extension().map_or(false, |x| x.to_ascii_lowercase() != "zip") {
//         return Err(RustiqueError::SimpleError(format!("Skipping non-zip file: {}", entry.display())));
//     }
//
//     let file  = File::open(&entry)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Failed to open {:?}: {}", entry.file_name(), e),
//             source: e,
//         })?;
//
//
//     let mut archive = ZipArchive::new(file)
//         .map_err(|e| RustiqueError::ZipError {
//             context: format!("Failed to open zip archive {:?}: {}", entry.file_name(),e),
//             source: e
//         })?;
//
//     let mut mod_info_file = archive.by_name("modinfo.json")
//         .map_err(|e| RustiqueError::ZipError {
//             context: format!("Failed to find modinfo.json in {:?}: {}", entry.file_name(),e),
//             source: e
//         })?;
//
//     let mut mod_info_contents = String::new();
//     mod_info_file.read_to_string(&mut mod_info_contents)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Failed to read modinfo.json in {:?}", entry.file_name()),
//             source: e,
//         })?;
//
//     let mod_info = serde_json5::from_str::<ModInfo>(&mod_info_contents)
//         .map_err(|e: serde_json5::Error| RustiqueError::JsonError {
//             context: format!("Failed to parse json in {}", entry.file_name().unwrap_or_default().to_string_lossy()),
//             source: e
//         })?;
//
//     Ok(mod_info)
// }
//
// pub fn extract_all_mods_metadata(mod_dir: &PathBuf) -> Result<HashMap<ModFileName, ModInfo>, RustiqueError> {
//
//     let dir = fs::read_dir(mod_dir)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Can't read mod_dir: {}", mod_dir.to_string_lossy()),
//             source: e,
//         })?;
//
//     let entries_vec: Vec<DirEntry> = dir.filter_map(|e| e.ok()).collect();
//     let mods = Arc::new(Mutex::new(HashMap::<ModFileName, ModInfo>::new()));
//
//     let notify_of_unzipped_mods = match get_config().read() {
//         Ok(config) => config.notify_of_unzipped_mods,
//         Err(e) => {
//             error!("Config error: {}", e.to_string());
//             false
//         }
//     };
//
//     entries_vec.par_iter().for_each(|entry| {
//         let filename = entry.file_name().to_string_lossy().to_string();
//         match (|| -> Result<ModInfo, RustiqueError> {
//             extract_zip_metadata(entry.path())
//         })() {
//             Ok(mod_info) => {mods.lock().unwrap().insert(filename, mod_info);}
//             Err(e) =>  {
//
//                 // verify_dir_is_mod(entry.path()) if true then display message
//                 if matches!(e, RustiqueError::ModNotZipped(_)) && notify_of_unzipped_mods {
//                     eprintln!("{}",e.to_string().yellow());
//                 } else {
//                     debug!("{}", e.to_string().yellow());
//                 }
//             }
//         }
//     });
//
//     Ok(mods.lock().unwrap().clone())
// }
//
// pub fn delete_file(file: &Path) -> Result<(), RustiqueError> {
//     debug!("Trying to delete {}", file.display());
//     if file.exists() && !file.is_dir() {
//         Ok(fs::remove_file(&file)
//             .map_err(|e| RustiqueError::IoError {
//                 context: format!("Failed attempting to delete {}", file.file_name().unwrap().to_string_lossy()),
//                 source: e,
//             })?)
//     } else {
//         Err(RustiqueError::SimpleError(format!("File {} is no longer there!", file.display())))
//     }
// }
pub async fn download_mod(mod_dir: &PathBuf, download_url: &String, api_client: &ApiClient) -> Result<ModInfo, RustiqueError> {
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
    debug!("Trying to download url: {}", url.clone().to_string());

    // Retry logic - attempt download up to 3 times
    let max_retries = 3;
    let mut attempt = 0;
    let mut last_error = None;

    while attempt < max_retries {
        attempt += 1;
        debug!("Download attempt {} for {}", attempt, url);

        match download_and_verify(&url, &file_path, api_client).await {
            Ok(mod_info) => {
                debug!("Successfully downloaded {} on attempt {}", file_path.display(), attempt);
                return Ok(mod_info);
            },
            Err(e) => {
                warn!("Download attempt {} failed for {}: {}", attempt, url, e);

                // Clean up any partial downloads
                if file_path.exists() {
                    if let Err(clean_err) = tokio::fs::remove_file(&file_path).await {
                        warn!("Failed to clean up partial download {}: {}", file_path.display(), clean_err);
                    }
                }

                last_error = Some(e);

                // Add a small delay between retries
                if attempt < max_retries {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| RustiqueError::SimpleError("Maximum retries exceeded".to_string())))
}

pub async fn download_and_verify(url: &Url, file_path: &PathBuf, api_client: &ApiClient) -> Result<ModInfo, RustiqueError> {
    let response = api_client.get_request(&url.to_string()).await
        .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;

    // Check if we got a successful response
    if !response.status().is_success() {
        return Err(RustiqueError::SimpleError(
            format!("Server returned error status: {}", response.status())
        ));
    }

    // Get the full response body
    let bytes = response.bytes().await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failure reading response from API {}", url.to_string()),
            source: std::io::Error::new(std::io::ErrorKind::Other, e),
        })?;

    // Verify we have actual content
    if bytes.is_empty() {
        return Err(RustiqueError::SimpleError("Downloaded file is empty".to_string()));
    }

    // Create and write to temp file first
    let temp_file_path = file_path.with_extension("tmp");

    let mut file = tokio::fs::File::create(&temp_file_path).await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Unable to create temp file {}", temp_file_path.to_string_lossy()),
            source: e
        })?;

    file.write_all(&bytes).await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failure while writing to file {}", temp_file_path.to_string_lossy()),
            source: e
        })?;

    // Ensure all data is written to disk
    file.sync_all().await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to flush file data for {}", temp_file_path.to_string_lossy()),
            source: e
        })?;

    // Close the file
    drop(file);

    // Pre-verify the zip file before extracting metadata
    verify_zip_file(&temp_file_path)?;

    // Rename temp file to final file
    tokio::fs::rename(&temp_file_path, file_path).await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to rename temp file to {}", file_path.to_string_lossy()),
            source: e
        })?;

    debug!("File downloaded to {}", file_path.display());

    // Extract metadata from the downloaded file
    extract_zip_metadata(file_path.clone())
}

pub fn verify_zip_file(file_path: &PathBuf) -> Result<(), RustiqueError> {
    // Open and verify the zip file integrity
    let file = File::open(file_path)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to open file for verification: {}", file_path.to_string_lossy()),
            source: e,
        })?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Invalid zip file: {}", file_path.to_string_lossy()),
            source: e
        })?;

    // Check that the archive contains at least one file
    if archive.len() == 0 {
        return Err(RustiqueError::SimpleError(format!("Zip file is empty: {}", file_path.to_string_lossy())));
    }

    // Verify we can access the modinfo.json
    archive.by_name("modinfo.json")
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Missing modinfo.json in zip: {}", file_path.to_string_lossy()),
            source: e
        })?;

    Ok(())
}
// pub async fn download_mod(mod_dir: &PathBuf, download_url: &String, api_client: &ApiClient) -> Result<ModInfo, RustiqueError> {
//     let filename_before = &download_url.split('=').last().unwrap();
//     let file_path_before = PathBuf::from(mod_dir.clone().join(filename_before));
//     // Replace any spaces in the downloaded file with _ . This makes it easier to process later
//     let filename_fix = mod_dir.clone().join(filename_before).to_string_lossy().replace(" ", "_");
//     let file_path = PathBuf::from(filename_fix);
//
//     if file_path.exists() && file_path_before.exists() {
//         return Err(RustiqueError::SimpleError(format!("File {} already exists.", file_path.display())))
//     }
//
//     let url = Url::parse(download_url.as_str())
//         .map_err(|e| RustiqueError::UrlParseError(e))?;
//     debug!("Trying to download url: {}", url.clone().to_string());
//
//     let response = api_client.get_request(&url.to_string()).await
//         .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
//
//     let bytes = response.bytes().await
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Failure reading response from API {}", download_url.red()),
//             source: std::io::Error::new(std::io::ErrorKind::Other, e),
//         })?;
//
//     let mut file = tokio::fs::File::create(&file_path).await
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Unable to create file {}", file_path.to_string_lossy()),
//             source: e
//         })?;
//
//     file.write_all(&bytes).await
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Failure while writing to byte array for {}", file_path.to_string_lossy()),
//             source: e
//         })?;
//
//     debug!("File downloaded to {}", file_path.display());
//
//     // Assuming extract_zip_metadata is synchronous
//     Ok(extract_zip_metadata(file_path)?)
// }
//

// pub fn download_mod(mod_dir: &PathBuf, download_url: &String, api_client: &ApiClient) -> Result<ModInfo, RustiqueError> {
//
//     let filename_before = &download_url.split('=').last().unwrap();
//     let file_path_before = PathBuf::from(mod_dir.clone().join(filename_before));
//
//     // Replace any spaces in the downloaded file with _ . This makes it easier to process later
//     let filename_fix = mod_dir.clone().join(filename_before).to_string_lossy().replace(" ", "_");
//     let file_path = PathBuf::from(filename_fix);
//
//     if file_path.exists() && file_path_before.exists() {
//         return Err(RustiqueError::SimpleError(format!("File {} already exists.", file_path.display())))
//     }
//
//     let url = Url::parse(download_url.as_str())
//         .map_err(|e| RustiqueError::UrlParseError(e))?;
//
//     debug!("Trying to download url: {}", url.clone().to_string());
//     let response = api_client.get_request(&url.to_string())
//         .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
//
//     let mut bytes: Vec<u8> = Vec::new();
//
//     response.into_body().into_reader().read_to_end(&mut bytes)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Failure reading response from API {}", download_url.red()),
//             source: e,
//         })?;
//
//     let mut file = File::create(&file_path)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Unable to create file {}", file_path.to_string_lossy()),
//             source: e
//         })?;
//
//     file.write_all(&bytes)
//         .map_err(|e| RustiqueError::IoError {
//             context: format!("Failure while writing to byte array for {}", file_path.to_string_lossy()),
//             source: e
//         })?;
//
//     debug!("File downloaded to {}", file_path.display());
//
//     Ok(extract_zip_metadata(file_path)?)
// }
//

// Replaces all instances of the newline and tab character from text, as well as excessive spaces.
// This is a fix for https://github.com/Tekunogosu/Rustique/issues/3
pub fn sanitize_string(string: &str) -> String {
    let re = Regex::new(r"[\n\t ]+").unwrap();
    re.replace_all(string, " ").to_string()
}

pub fn elapsed_footer(start_time: Instant, operation: &str) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_BORDERS_ONLY)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    let elapsed = format!("{:.2}s", start_time.elapsed().as_secs_f64());
    // let out_str = format!("{} {} {}{}", operation.bright_green().bold(),"operation took:".bright_green().bold(), elapsed.bright_purple(), "s".bright_yellow());
    let operation_str = format!("{} {}", operation, "operation completed: ");
    let mut row = Row::new();

    row.add_cell(Cell::new(operation_str.as_str()).fg(Color::Green).add_attribute(Attribute::Bold));
    row.add_cell(Cell::new(elapsed.as_str()).fg(Color::Magenta).add_attribute(Attribute::Bold));

    table.add_row(row);

    println!("{}", table);
}

pub fn notice(message: &str, fg_color: Option<Color>, attributes: Vec<Attribute>) {
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY).apply_modifier(UTF8_ROUND_CORNERS);

    let mut cell = Cell::new(message);

    if let Some(color) = fg_color {
        cell = cell.fg(color);
    }

    if !attributes.is_empty() {
        for attribute in attributes {
            cell = cell.add_attribute(attribute);
        }
    }

    cell = cell.set_alignment(CellAlignment::Center);

    let mut row = Row::new();
    row.add_cell(cell);

    table.add_row(row);
    println!("{}", table);
}

pub struct CellData {
    text: String,
    attributes: Vec<Attribute>,
    color: Option<Color>,
}

impl CellData {
    pub fn new(text: String, color: Option<Color>, attributes: Vec<Attribute>) -> CellData {
        Self {
            text,
            attributes,
            color,
        }
    }
}

pub fn display_table(row_data: Vec<(CellData, CellData)>, table_style: Option<&str>) {
    let style = table_style.unwrap_or(UTF8_BORDERS_ONLY);
    let mut table = Table::new();
    table.load_preset(style).apply_modifier(UTF8_ROUND_CORNERS);

    let mut rows: Vec<Row> = Vec::new();

    for (l_col, r_col) in row_data {
        let mut row = Row::new();
        row.add_cell(construct_cell(l_col));
        row.add_cell(construct_cell(r_col));
        rows.push(row);
    }

    table.add_rows(rows);

    println!("{}", table);
}

pub fn construct_cell(dt: CellData) -> Cell {
    let mut cell = Cell::new(dt.text);

    if let Some(color) = dt.color {
        cell = cell.fg(color);
    }

    for attr in dt.attributes {
        cell = cell.add_attribute(attr);
    }

    cell
}
pub fn command_output(option: String, val: String) -> (CellData, CellData) {
    (
        CellData::new(option, Some(Color::Green), vec![Attribute::Bold]),
        CellData::new(val, Some(Color::Magenta), vec![Attribute::Bold]),
    )
}

