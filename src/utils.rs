use crate::aliases::{ModFileName, ModID};
use crate::api::api_structs::ModInfo;
use crate::commands::sync::ModSyncInfo;
use crate::config_manager::get_config;
use crate::install_manager::{Install, Installed};
use crate::rustique_errors::RustiqueError;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use owo_colors::OwoColorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_BORDERS_ONLY, UTF8_FULL_CONDENSED};
use comfy_table::{Attribute, Cell, CellAlignment, Color, Row, Table};
use dirs::home_dir;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{DirEntry, File};
use std::io::{Read};
use std::path::{Path, PathBuf};
use std::time::{Instant};
use std::{fs};
use comfy_table::ContentArrangement::Dynamic;
use tracing::{debug, error, info};
use zip::ZipArchive;
use crate::config_structs::{CellAttr, CellColor};

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

pub fn elapsed_footer(start_time: Instant, operation: &str) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_BORDERS_ONLY)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(Dynamic);

    let elapsed = format!("{:.2}s", start_time.elapsed().as_secs_f64());
    // let out_str = format!("{} {} {}{}", operation.bright_green().bold(),"operation took:".bright_green().bold(), elapsed.bright_purple(), "s".bright_yellow());
    let operation_str = format!("{} {}", operation, "operation completed: ");
    let mut row = Row::new();

    row.add_cell(Cell::new(operation_str.as_str()).fg(Color::Green).add_attribute(Attribute::Bold));
    row.add_cell(Cell::new(elapsed.as_str()).fg(Color::Magenta).add_attribute(Attribute::Bold));

    table.add_row(row);

    println!("{table}");
}

pub fn notice(message: &str, fg_color: Option<Color>, attributes: Vec<Attribute>) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_BORDERS_ONLY)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(Dynamic);

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
    println!("{table}");
}

pub struct RustiqueMessage {
    pub header: Option<CellData>,
    pub message: Vec<CellData>
}

pub fn rustique_message(rustique_message: RustiqueMessage) {
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(Dynamic);

    if rustique_message.header.is_some() {
        let header_data = rustique_message.header;
        if header_data.is_some() {
            let header_data = header_data.unwrap_or_default();
            let mut h_cell = Cell::new(header_data.text);
            if !header_data.attributes.is_empty() {
                for attribute in &header_data.attributes {
                    h_cell = h_cell.add_attribute(*attribute);
                }
            }
            h_cell = h_cell.fg(header_data.color.unwrap_or(Color::Green))
                .set_alignment(header_data.alignment.unwrap_or(CellAlignment::Center));

            let mut row = Row::new();
            row.add_cell(h_cell);

            table.set_header(row);
        }
    }

    let rows: Vec<Row> = rustique_message.message.iter().map(|message_data|{
        let mut cell = Cell::from(message_data.text.clone());

        if !message_data.attributes.is_empty() {
            for attr in &message_data.attributes {
                cell = cell.add_attribute(*attr);
            }
        }
        cell = cell.fg(message_data.color.unwrap_or(Color::Yellow))
            .set_alignment(message_data.alignment.unwrap_or(CellAlignment::Center));

        let mut row = Row::new();
        row.add_cell(cell);
        row
    }).collect();

    table.add_rows(rows);

    println!("{table}");
}

#[derive(Default)]
pub struct CellData {
    pub(crate) text: String,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) color: Option<Color>,
    pub(crate) alignment: Option<CellAlignment>
}

impl CellData {
    pub fn new(text: String, color: Option<Color>, attributes: Vec<Attribute>, alignment: Option<CellAlignment>) -> CellData {
        Self {
            text,
            attributes,
            color,
            alignment
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

    println!("{table}");
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
        CellData::new(option, Some(Color::Yellow), vec![Attribute::Bold], None),
        CellData::new(val, Some(Color::Magenta), vec![Attribute::Bold], None),
    )
}

pub fn display_installation_results(mods_processed: Vec<Installed>) {
    let (mut successful, mut failed): (Vec<Installed>, Vec<Installed>) = mods_processed.into_iter().partition(|m| m.success);

    let mut s_table = Table::new();
    s_table.load_preset(UTF8_FULL_CONDENSED).apply_modifier(UTF8_ROUND_CORNERS);
    let mut f_table = s_table.clone();


    if !successful.is_empty() {
        let mut sh_row = Row::new();
        sh_row.add_cell(Cell::new("Successfully Installed".to_string()).fg(Color::Green).add_attribute(Attribute::Bold).set_alignment(CellAlignment::Center));
        s_table.set_header(sh_row);

        fill_table_body(&mut successful, &mut s_table, Color::Green, Color::Magenta);

        println!("{s_table}");

        display_table(vec![command_output("Total mods Installed".to_string(), successful.len().to_string())], None);
    }

    if !failed.is_empty() {
        let mut fh_row = Row::new();
        fh_row.add_cell(Cell::new("Failed to Install".to_string()).fg(Color::Red).add_attribute(Attribute::Bold).set_alignment(CellAlignment::Center));
        f_table.set_header(fh_row);

        fill_table_body(&mut failed, &mut f_table, Color::Red, Color::Magenta);

        println!("{f_table}");
    }
}

fn fill_table_body(list: &mut [Installed], table: &mut Table, l_color: Color, r_color: Color) {
    list.sort_by(|a,b| a.mod_name.cmp(&b.mod_name));

    for m in list {
        let mut row = Row::new();
        row.add_cell(Cell::new(m.mod_name.clone()).fg(l_color).set_alignment(CellAlignment::Left));
        row.add_cell(Cell::new(m.install_version.clone()).fg(r_color).set_alignment(CellAlignment::Left));
        table.add_row(row);
    }
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

pub fn prep_cell(text: &str, color: Option<CellColor>, attribute: Option<CellAttr>, delimiter: Option<char>) -> Cell {
    let mut cell = Cell::from(text);

    if color.is_some() {
        cell = cell.fg(Color::from(color.unwrap_or(CellColor::Reset)));
    }

    // TODO: Add actual attribute type so any Comfy_table attribute can be used
    // For now we limit the usable attributes
    if attribute.is_some() {
        cell = cell.add_attribute(Attribute::from(attribute.unwrap_or(CellAttr::NoHidden)));
    }

    if delimiter.is_some() {
        cell = cell.set_delimiter(delimiter.unwrap_or(' '));
    }

    cell
}