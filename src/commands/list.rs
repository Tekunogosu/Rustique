use std::collections::HashSet;
use crate::api_structs::ModInfo;
use crate::commands::sync::{parse_sync_file, RustiqueSyncJson};
use crate::utils::{RustiqueOptions, extract_all_mods_metadata, extract_zip_metadata, find_missing_dependencies, sanitize_string};
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
use regex::Regex;
use ureq::get;
use zip::ZipArchive;
use crate::aliases::ModID;
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
        .add_cell(Cell::new("Dependencies").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("Missing Dependencies").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("Description").add_attribute(Attribute::Bold).fg(Color::Blue));
        // .add_cell(Cell::new("Website").add_attribute(Attribute::Bold).fg(Color::Blue));

    table.add_row(header);

    table
}

// TODO:: Should we handle mods that are in directories and not .zip files
pub fn list_installed(mod_dir: &PathBuf, only_updated: bool) -> Result<(), RustiqueError> {
    // TODO: check which platform we are on

    // check for sync data so we can show latest version
    let sync_data = parse_sync_file(mod_dir);

    let mut table = setup_table_from_sync(&sync_data);

    let sync_data_unwrapped = sync_data.as_ref().ok();

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

    let mod_id_list: HashSet<ModID> = metadata.clone()
        .into_iter().map(|mod_info| mod_info.mod_id.clone()).collect();

    metadata.into_iter().for_each(|mod_info| {
        let mut row = Row::new();
        row.add_cell(Cell::new(&mod_info.name).fg(Color::Yellow))
            .add_cell(Cell::new(&mod_info.mod_id));

        let mut installed_version_cell = Cell::new(mod_info.version.clone().unwrap_or_default()).add_attribute(Attribute::Dim);
        let mut latest_version_cell: Cell = Cell::new("N/A");
        if let Some(data) = &sync_data_unwrapped {
            if let Some(sync) = data.rustique_sync.get(mod_info.mod_id.as_str()) {


                let latest_version = sync.latest_known_version.to_string();
                // add the installed version that we see from the sync file as its been parse at this point

                let current_version = sync.installed_version.to_string();
                installed_version_cell = Cell::new(&current_version).add_attribute(Attribute::Dim);

                let mut cell = Cell::new(&sync.latest_known_version.to_string());

                if latest_version.eq(current_version.to_string().as_str()) {
                    cell = cell.fg(Color::Green).add_attribute(Attribute::Dim);
                } else {
                    cell = cell.add_attribute(Attribute::Bold).fg(Color::Red);
                }
                latest_version_cell = cell;
            }
        }
        row.add_cell(installed_version_cell);

        if sync_data.as_ref().is_ok() {
            row.add_cell(latest_version_cell);
        }

        let missing_dependencies = find_missing_dependencies(mod_info.dependencies.clone(), Option::from(&mod_id_list));

        row
            .add_cell(
            Cell::new(
                find_missing_dependencies(mod_info.dependencies.clone(), None).join(",")
            )
        ).add_cell(Cell::new(
                missing_dependencies.join(", ").as_str()
        ).fg(Color::Red).add_attribute(Attribute::SlowBlink).add_attribute(Attribute::Bold))
            .add_cell(Cell::new(sanitize_string(mod_info.description.as_ref().unwrap_or(&"".to_string())))
        );

        table.add_row(row);
    });

    println!("{}", table);

    Ok(())
}


