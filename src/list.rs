use crate::api_structs::ModInfo;
use crate::sync::{parse_sync_file, RustiqueSyncJson};
use crate::utils::{RustiqueOptions, extract_all_mods_metadata, extract_zip_metadata};
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Row, Table};
use rayon::prelude::*;
use std::error::Error;
use std::fmt::format;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::{Read, stdin};
use std::ops::Add;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use colored::Colorize;
use ureq::get;
use zip::ZipArchive;
use crate::rustique_errors::RustiqueError;

fn setup_table_from_sync(sync_data: &Result<RustiqueSyncJson, RustiqueError>) -> Table {
    let mut table = Table::new();

    table.set_content_arrangement(ContentArrangement::Dynamic);

    let mut header = Row::new();
    header
        .add_cell(Cell::new("Name").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("ModID").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("Version").add_attribute(Attribute::Bold).fg(Color::Blue));

    if sync_data.is_ok() {
        header.add_cell(Cell::new("Latest Version").add_attribute(Attribute::Bold).fg(Color::Blue));
    }

    header
        .add_cell(Cell::new("Missing Dependencies").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("Description").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("Website").add_attribute(Attribute::Bold).fg(Color::Blue));

    table.add_row(header);

    table
}

// TODO:: Should we handle mods that are in directories and not .zip files
pub fn list_installed(mod_dir: &PathBuf, only_updated: bool) -> Result<(), RustiqueError> {
    // TODO: check which platform we are on

    // check for sync data so we can show latest version
    let sync_data = parse_sync_file(mod_dir);

    let mut table = setup_table_from_sync(&sync_data);

    let sync_data_unwrapped = sync_data.ok();

    let metadata = extract_all_mods_metadata(&mod_dir)?;
    let mut metadata: Vec<&ModInfo> = metadata.values().collect();

    metadata.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let metadata: Vec<&ModInfo> = if only_updated {
        metadata.into_iter().filter(|mod_info| {
            if let Some(data) = &sync_data_unwrapped {
                if let Some(sync) = data.rustique_sync.get(mod_info.mod_id.as_str()) {
                    let latest = sync.latest_known_version.to_string();
                    let current = sync.installed_version.to_string();
                    return &latest != &current;
                }
            }
            false
        }).collect::<Vec<&ModInfo>>()
    } else {
        metadata
    };

    if metadata.is_empty() {
        eprintln!("{}","All mods are up to date!".green().bold());
        return Ok(());
    }

    let mod_id_list: Vec<String> = metadata.clone().into_iter().map(|mod_info| mod_info.mod_id.clone()).collect();

    metadata.into_iter().for_each(|mod_info| {
        let mut row = Row::new();
        row.add_cell(Cell::new(&mod_info.name).add_attribute(Attribute::Bold).fg(Color::Blue))
            .add_cell(Cell::new(&mod_info.mod_id))
            .add_cell(Cell::new(
                mod_info.version.as_ref().unwrap_or(&"".to_string()),
            ));

        if let Some(data) = &sync_data_unwrapped {
            if let Some(sync) = data.rustique_sync.get(mod_info.mod_id.as_str()) {
                let latest_version = sync.latest_known_version.to_string();
                let current_version = mod_info.version.as_ref().unwrap_or(&latest_version);

                let mut cell = Cell::new(&sync.latest_known_version.to_string());

                if &latest_version == current_version {
                    cell = cell.add_attribute(Attribute::Bold).fg(Color::Green);
                } else {
                    cell = cell.fg(Color::Red);
                }
                row.add_cell(cell);
            } else {
                row.add_cell(Cell::new("N/A"));
            }
        }

        let missing_dependencies: Vec<String> = mod_info.dependencies.clone()
            .unwrap().keys()
            .filter(|e|e.to_lowercase().ne("game") && !mod_id_list.contains(e))
            .cloned().collect();


        row.add_cell(Cell::new(
            missing_dependencies.join(", ").as_str()
        ).fg(Color::Red).add_attribute(Attribute::SlowBlink).add_attribute(Attribute::Bold))
            .add_cell(Cell::new(
            mod_info.description.as_ref().unwrap_or(&"".to_string()),
        )).add_cell(Cell::new(
            mod_info.website.as_ref().unwrap_or(&"".to_string()),
        ));

        table.add_row(row);
    });

    println!("{}", table);

    Ok(())
}
