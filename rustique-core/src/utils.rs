use crate::aliases::{ModFileName, ModID, ModVersion};
use crate::api::api_structs::{ModApi, ModInfo};
use crate::config::config_manager::Config;
use crate::config::config_manager::get_config;
use crate::consts::{FILE_GAME_VERSION_SYNC, FILE_MODINFO_JSON, FILE_RUSTIQUE_SYNC};
use crate::information_utils::{CellData, display_table, notice};
use crate::install_manager::{Install, Installed};
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::{PathRef, StrRef};
use crate::version_management::parse_version;
use async_zip::tokio::read::fs::ZipFileReader;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use comfy_table::{Color, Attribute};
use comfy_table::presets::UTF8_HORIZONTAL_ONLY;
use dirs::home_dir;
use futures::{StreamExt, stream};
use owo_colors::OwoColorize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::exit;
use semver::VersionReq;
use serde_json::to_string_pretty;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, error, info, warn};
use crate::symlink_manager::SymlinkManager;
use crate::sync_structs::{GameVersionSync, ModSyncInfo};

pub fn get_current_time() -> String {
    let datetime: DateTime<Utc> = Utc::now();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

pub fn timestamp_older_than(num_hours: i64, timestamp: &str) -> bool {
    let naive_dt = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M")
        .map_err(|e| error!("{}", e))
        .unwrap_or_default();
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

pub async fn extract_zip_metadata<T>(
    entry: impl PathRef,
    inner_file: &str,
) -> Result<T, RustiqueError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let entry = entry.as_ref();
    // This function doesn't need async as it's doing synchronous file operations
    if entry.is_dir() {
        return Err(RustiqueError::ModNotZipped(entry.display().to_string()));
    }
    if entry
        .extension()
        .is_some_and(|x| !x.eq_ignore_ascii_case("zip"))
    {
        return Err(RustiqueError::SimpleError(format!(
            "Skipping non-zip file: {}",
            entry.display()
        )));
    }

    let archive = ZipFileReader::new(entry)
        .await
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Failed to open zip archive {:?}: {}", entry.file_name(), e),
            source: e,
        })?;

    // Locate the file we want
    let entry_index = archive
        .file()
        .entries()
        .iter()
        .position(|e| {
            e.filename()
                .as_str()
                .unwrap()
                .eq_ignore_ascii_case(inner_file)
        })
        .ok_or_else(|| RustiqueError::ZipError {
            context: format!("Failed to find {} in {:?}", inner_file, entry.file_name()),
            source: async_zip::error::ZipError::UnableToLocateEOCDR,
        })?;

    let mut entry_reader =
        archive
            .reader_with_entry(entry_index)
            .await
            .map_err(|e| RustiqueError::ZipError {
                context: format!("Failed to read {} in {:?}", inner_file, entry.file_name()),
                source: e,
            })?;

    // read the content of the file inner_file
    let mut mod_info_contents = String::new();
    entry_reader
        .read_to_string_checked(&mut mod_info_contents)
        .await
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Failed to read {} in {:?}", inner_file, entry.file_name()),
            source: e,
        })?;

    let mod_info = if inner_file.to_lowercase().ends_with(".json") {
        serde_json5::from_str::<T>(&mod_info_contents).map_err(|e: serde_json5::Error| {
            RustiqueError::JsonError {
                context: format!(
                    "Failed to parse json in {}",
                    entry.file_name().unwrap_or_default().to_string_lossy()
                ),
                source: e,
            }
        })?
    } else if inner_file.to_lowercase().ends_with(".toml") {
        toml::from_str::<T>(&mod_info_contents).map_err(|e| RustiqueError::TomlError {
            context: format!(
                "Failed to parse toml in {}",
                entry.file_name().unwrap_or_default().to_string_lossy()
            ),
            source: e,
        })?
    } else {
        return Err(RustiqueError::SimpleError(format!(
            "Unsupported file format {inner_file}"
        )));
    };

    Ok(mod_info)
}

pub async fn extract_all_mods_metadata(
    mod_dir: impl PathRef,
    ignore_symlink: bool,
) -> Result<HashMap<ModFileName, ModInfo>, RustiqueError> {
    let mod_dir = mod_dir.as_ref();
    let mut dir = tokio::fs::read_dir(mod_dir)
        .await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Can't read mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;
    let mut entries = Vec::new();

    while let Some(entry) = dir.next_entry().await? {
        entries.push(entry);
    }

    let concurrent_limit = num_cpus::get();

    let config = get_config().read().await;
    // Create a local copy of the data needed so we can drop the config
    let notif_unzipped_mods = config.notify_of_unzipped_mods;
    drop(config); // manually drop the config, we don't actually need it anymore

    let results: Vec<(ModFileName, ModInfo)> = stream::iter(entries)
        // This is to ignore modpack mods when using normal rustique commands while a modpack is enabled
        .filter(|e| futures::future::ready(!(ignore_symlink && SymlinkManager::exists(e.path()))))
        .map(|entry| async move {
            let filename = entry.file_name().to_string_lossy().to_string();
            extract_zip_metadata::<ModInfo>(&entry.path(), FILE_MODINFO_JSON)
                .await
                .map(|mod_info| (filename, mod_info))
                .inspect_err(|e| {
                    if matches!(e, RustiqueError::ModNotZipped(_)) && notif_unzipped_mods {
                        println!("{}", e.to_string().yellow());
                    } else {
                        debug!("{}", e.to_string().yellow());
                    }
                })
                .ok()
        })
        .buffer_unordered(concurrent_limit)
        .filter_map(futures::future::ready)
        .collect()
        .await;

    Ok(results.into_iter().collect())
}

// TODO: Decide if this function is needed
#[allow(dead_code)]
pub async fn verify_zip_file(file_path: impl PathRef) -> Result<(), RustiqueError> {
    // Open and verify the zip file integrity
    let file_path = file_path.as_ref();

    let archive = ZipFileReader::new(file_path)
        .await
        .map_err(|e| RustiqueError::ZipError {
            context: format!("Invalid zip file: {}", file_path.to_string_lossy()),
            source: e,
        })?;

    // Check that the archive contains at least one file
    if archive.file().entries().is_empty() {
        return Err(RustiqueError::SimpleError(format!(
            "Zip file is empty: {}",
            file_path.to_string_lossy()
        )));
    }

    Ok(())
}

pub async fn delete_file(file: impl PathRef) -> Result<(), RustiqueError> {
    let file = file.as_ref();
    debug!("Trying to delete {}", file.display());
    if file.exists() && !file.is_dir() {
        tokio::fs::remove_file(file)
            .await
            .map_err(|e| RustiqueError::IoError {
                context: format!(
                    "Failed attempting to delete {}",
                    file.file_name().unwrap().to_string_lossy()
                ),
                source: e,
            })
    } else {
        Err(RustiqueError::SimpleError(format!(
            "File {} is no longer there!",
            file.display()
        )))
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
    gather_missing_dependencies(installed_mods, &[], &BTreeMap::new())
}

pub fn gather_missing_dependencies<V: AsRef<[ModID]>>(
    installed_mods: &HashMap<ModFileName, ModInfo>,
    mods_requested: V,
    sync_data: &BTreeMap<ModID, ModSyncInfo>,
) -> Vec<Install> {
    // if there are reports of slowness is this section .values().par_bridge()...flat_map_iter() could be used to speed it up
    // this is prob not an issue even with a lot of mods as the data is all in memory at this point
    let id_vec: Vec<ModID> = sync_data
        .keys()
        .map(|m| {
            let p = split_modid_version(m).0; // split the version from the mod_id
            p.clone()
        })
        .collect();

    let mods_requested = mods_requested.as_ref();

    installed_mods
        // .values()
        .iter()
        .filter(|(_, mod_info)| {
            mods_requested.is_empty() || mods_requested.contains(&mod_info.mod_id)
        })
        .flat_map(|(mod_filename, mod_info)| {
            mod_info
                .dependencies
                .iter()
                .filter_map(|(mod_id, version)| {
                    if !mod_id.contains("game")
                        && !mod_id.contains("survival")
                        && !mod_id.contains("creative")
                        && !id_vec.contains(mod_id)
                    {
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
                })
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect()
}

pub async fn parse_json_file<T>(file_path: impl PathRef) -> Result<T, RustiqueError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let file_path = file_path.as_ref();
    let filename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut file = File::open(file_path)
        .await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Unable to open {filename}"),
            source: e,
        })?;

    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)
        .await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failure while reading from file {filename}"),
            source: e,
        })?;

    let json = serde_json5::from_str::<T>(&file_contents).map_err(|e| {
        let sync_error = if filename.eq(FILE_RUSTIQUE_SYNC) {
            format!(
                "{} {} {}",
                "(Run".yellow(),
                "Rustique sync".blue(),
                "to repopulate the sync file and resolve this message)".yellow()
            )
        } else {
            String::new()
        };
        RustiqueError::JsonError {
            context: format!("Json parsing Error for {filename} {sync_error}"),
            source: e,
        }
    })?;

    Ok(json)
}

pub async fn write_json_file(
    file_path: impl PathRef,
    json: String,
    config_dir: impl PathRef,
) -> Result<(), RustiqueError> {
    let (file_path, config_dir) = (file_path.as_ref(), config_dir.as_ref());
    let mut open_file = File::create(file_path)
        .await
        .map_err(|e| RustiqueError::IoError {
            context: format!(
                "Error writing sync mod search file to config dir: {}",
                config_dir.to_string_lossy()
            ),
            source: e,
        })?;
    AsyncWriteExt::write_all(&mut open_file, json.as_bytes()).await?;

    Ok(())
}

pub async fn sorted_game_versions() -> Vec<String> {
    let version_file_path = Config::get_path().join(FILE_GAME_VERSION_SYNC);

    let mut versions = if version_file_path.exists() {
        match parse_json_file::<GameVersionSync>(&version_file_path).await {
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

    versions.sort_by(|v1, v2| {
        let v1_p = lenient_semver::parse(v1).unwrap();
        let v2_p = lenient_semver::parse(v2).unwrap();
        v1_p.cmp(&v2_p)
    });

    versions.reverse();
    versions
}

/// Returns mod_id as lowercase
pub fn find_mod_id<V: AsRef<[ModApi]>>(
    mod_name: &String,
    mod_filename: &ModFileName,
    mods_search_data: V,
) -> Result<String, RustiqueError> {
    let mods_search_data = mods_search_data.as_ref();
    info!(
        "{} has an empty mod id, attempting locate mod id...",
        mod_filename
    );
    let res: Vec<ModApi> = mods_search_data
        .iter()
        .filter(|mod_search| match &mod_search.name {
            Some(name) => mod_name.eq_ignore_ascii_case(name),
            None => mod_search.mod_id_strs.contains(mod_name),
        })
        .cloned()
        .collect();

    if res.is_empty() || res.len() > 1 {
        // no mods match
        warn!(
            "Unable to determine the mod_id for {} - {}.\n\r\t Their modinfo.json is malformed and no information provided allowed Rustique to determine it.\n\r\t \
                     Please contact the author to correct their modinfo.json file",
            mod_name.bright_red().bold(),
            mod_filename.bright_red().bold()
        );
        Err(RustiqueError::SimpleError(format!(
            "Unable to locate mod_id for {mod_name}"
        )))
    } else {
        Ok(res[0].mod_id.to_string().to_lowercase())
    }
}

/// Removes older files after updates
///
/// processed_install: Vec<Installed>
pub async fn remove_older_files(processed_install: &[Installed]) -> Result<(), RustiqueError> {
    for mod_installed in processed_install {
        if let (Some(old), Some(new)) = (
            &mod_installed.old_file_path,
            &mod_installed.installed_file_path,
        ) {
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

pub async fn backup_older_files(processed_install: &[Installed]) -> Result<(), RustiqueError> {
    let config = get_config().read().await;
    let backup_dir = Path::new(&config.backup_mods_dir);

    if !backup_dir.exists() {
        tokio::fs::create_dir_all(backup_dir).await?;
    }

    for m in processed_install {
        if let Some(old_file_name) = &m
            .old_file_path
            .clone()
            .unwrap_or(PathBuf::new())
            .file_name()
        {
            tokio::fs::copy(
                &m.old_file_path.clone().unwrap_or(PathBuf::new()),
                backup_dir.join(old_file_name),
            )
            .await?;
        }
    }

    display_table(
        vec![(
            CellData::new(
                "Updated mods have been backed up to:".into(),
                Some(Color::Green),
                vec![],
                None,
            ),
            CellData::new(
                format!("{}", backup_dir.display()),
                Some(Color::Magenta),
                vec![],
                None,
            ),
        )],
        Some(UTF8_HORIZONTAL_ONLY),
    );

    Ok(())
}

pub fn split_modid_version(mod_id_str: impl StrRef) -> (ModID, Option<ModVersion>) {
    if let Some((modid, version)) = mod_id_str
        .as_ref()
        .strip_prefix("vintagestorymodinstall://")
        .unwrap_or(mod_id_str.as_ref())
        .split_once('@')
    {
         let version = if !has_semver_operator(version) {
            format!("={version}")
        } else { version.to_string() };

        let p_ver = match VersionReq::parse(&version) {
            Ok(v) => v,
            Err(_) => {
                notice(
                format!(
                    "{} - failed to parse {}, invalid semver version. See https://semver.org for valid semver standards",
                    mod_id_str.as_ref(), version
                ),
                Some(Color::Red),
                vec![Attribute::Bold],
            );
                exit(1)
            }
        };

        return (modid.to_string(), Some(p_ver.to_string()));
    }

    (mod_id_str.as_ref().to_string().to_lowercase(), None)
}

pub fn has_semver_operator(s: &str) -> bool {
    matches!(s.chars().next(), Some('^' | '<' | '>' | '=')) ||
    s.starts_with("<=") ||
    s.starts_with(">=")
}

pub fn format_for_csv(input: impl StrRef) -> String {
    let input = normalize_whitespace(input.as_ref());
    if input.contains(',') || input.contains('\n') || input.contains('\r') || input.contains('"') {
        // wrap the text in quotes and escape internal quotes with by doubling them
        format!("\"{}\"", input.replace(['"', '\n', '\r', ','], "\"\""))
    } else {
        input.to_string()
    }
}

pub fn normalize_whitespace(input: impl StrRef) -> String {
    input
        .as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn html_parse(input: &mut impl StrRef, width: usize) -> Result<String, RustiqueError> {
    html2text::from_read(&mut input.as_ref().as_bytes(), width)
        .map_err(|_| RustiqueError::SimpleError("html2txt failed".to_string()))
}

pub fn prettify<T>(data: T, command_type: impl StrRef) -> Result<String, RustiqueError>
where
    T: serde::Serialize {

    to_string_pretty(&data).map_err(|e| RustiqueError::JsonError {
        context: format!("Failure while making the {} json pretty", command_type.as_ref()),
        source: serde_json5::Error::from(std::io::Error::other(e)),
    })
}
