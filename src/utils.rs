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
use chrono::{DateTime, Local, NaiveDateTime, Utc, Duration, TimeZone};
use colored::Colorize;
use comfy_table::{Cell, Row, Table, Color, Attribute, CellAlignment, TableComponent};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_BORDERS_ONLY, UTF8_FULL, UTF8_FULL_CONDENSED, UTF8_HORIZONTAL_ONLY};
use dirs::home_dir;
use rayon::prelude::*;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use toml::value::Time;
use tracing::{debug, error, warn};
use tracing::span::Attributes;
use tracing_subscriber::fmt::time;
use url::Url;
use zip::result::ZipError;
use zip::ZipArchive;
use crate::aliases::{ModFileName, ModID, ModVersion};
use crate::api::client::ApiClient;
use crate::api::api_structs::ModInfo;
use crate::config_manager::get_config;
use crate::install_manager::Installed;
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
    let datetime: DateTime<Utc> = Utc::now();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

pub fn is_today(timestamp: &String) -> bool {
   let now = Utc::now().date_naive();
    if let Ok(ts) = NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M") {
        ts.date() == now
    } else {
        false
    }
}

pub fn timestamp_older_than(num_hours: i64, timestamp: &String) -> bool {

    let naive_dt = NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M").map_err(|e| {error!("{}", e)}).unwrap_or_default();
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
            return PathBuf::new().join(home).join(dir.strip_prefix("~/").unwrap());
        }
    }

    dir
}

// this function filters out any unwanted dependencies
pub fn find_missing_dependencies(
    dep_list: Option<HashMap<ModID, ModVersion>>,
    excluded_ids: Option<&HashSet<ModID>>,
) -> Vec<ModID> {
    let default_exclusions = ["game", "survival", "creative"];
    let empty_set :HashSet<ModID> = HashSet::new();
    let excluded = excluded_ids.unwrap_or(&empty_set);
    dep_list.unwrap_or_default()
        .keys()
        .filter(|mod_id|
            !default_exclusions.contains(&mod_id.to_lowercase().as_str())
            && !excluded.contains(&mod_id.to_lowercase().to_string())
        ).cloned().collect()
}


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

    let dir = fs::read_dir(mod_dir)
        .map_err(|e| RustiqueError::IoError {
            context: format!("Can't read mod_dir: {}", mod_dir.to_string_lossy()),
            source: e,
        })?;
    let entries_vec: Vec<DirEntry> = dir.filter_map(|e| e.ok()).collect();
    // let mods = Arc::new(Mutex::new(HashMap::<ModFileName, ModInfo>::new()));

    let notify_of_unzipped_mods = match get_config().read() {
        Ok(config) => config.notify_of_unzipped_mods,
        Err(e) => {
            error!("Config error: {}", e.to_string());
            false
        }
    };

    // Use Rayon for CPU-bound tasks (zip processing is CPU-bound)
    let results:Vec<(ModFileName, ModInfo)> = entries_vec.par_iter()
        .filter_map(|entry| {
            let filename = entry.file_name().to_string_lossy().to_string();
            match extract_zip_metadata(entry.path()) {
                Ok(mod_info) => Some((filename, mod_info)),
                Err(e) => {
                     if matches!(e, RustiqueError::ModNotZipped(_)) && notify_of_unzipped_mods {
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
    if archive.len() == 0 {
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
                acc.push_str(" ");
            }
            acc.push_str(word);
            acc
        })
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
        CellData::new(option, Some(Color::Blue), vec![Attribute::Bold]),
        CellData::new(val, Some(Color::Magenta), vec![Attribute::Bold]),
    )
}

pub fn display_installation_results(mods_processed: Vec<Installed>) {
    let (mut successful, mut failed): (Vec<Installed>, Vec<Installed>) = mods_processed.into_iter().partition(|m| m.success);

    let mut s_table = Table::new();
    s_table.load_preset(UTF8_FULL_CONDENSED).apply_modifier(UTF8_ROUND_CORNERS);
    let mut f_table = s_table.clone();


    if successful.len() > 0 {
        let mut sh_row = Row::new();
        sh_row.add_cell(Cell::new("Successfully Installed".to_string()).fg(Color::Green).add_attribute(Attribute::Bold).set_alignment(CellAlignment::Center));
        s_table.set_header(sh_row);

        fill_table_body(&mut successful, &mut s_table, Color::Green, Color::Magenta);

        println!("{}", s_table);

        display_table(vec![command_output("Total mods Installed".to_string(), successful.len().to_string())], None);
    }

    if failed.len() > 0 {
        let mut fh_row = Row::new();
        fh_row.add_cell(Cell::new("Failed to Install".to_string()).fg(Color::Red).add_attribute(Attribute::Bold).set_alignment(CellAlignment::Center));
        f_table.set_header(fh_row);

        fill_table_body(&mut failed, &mut f_table, Color::Red, Color::Magenta);

        println!("{}", f_table);
    }
}

fn fill_table_body(list: &mut Vec<Installed>, table: &mut Table, l_color: Color, r_color: Color) {
    list.sort_by(|a,b| a.mod_name.cmp(&b.mod_name));

    list.iter().for_each(|m|{
        let mut row = Row::new();
        row.add_cell(Cell::new(m.mod_name.clone()).fg(l_color).set_alignment(CellAlignment::Left));
        row.add_cell(Cell::new(m.install_version.clone()).fg(r_color).set_alignment(CellAlignment::Left));
        table.add_row(row);
    });
}

