use crate::api_structs::ModInfo;
use crate::sync::parse_sync_file;
use crate::utils::{RustiqueOptions, extract_all_mods_metadata, extract_zip_metadata};
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Row, Table};
use rayon::prelude::*;
use std::error::Error;
use std::fmt::format;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::{Read, stdin};
use std::sync::{Arc, Mutex};
use ureq::get;
use zip::ZipArchive;

// TODO:: Should we handle mods that are in directories and not .zip files
pub fn list_installed(rustique_options: RustiqueOptions) -> Result<(), Box<dyn Error>> {
    // TODO: check which platform we are on

    // check for sync data so we can show latest version
    let sync_data = parse_sync_file(rustique_options.clone().mod_dir.unwrap());

    // remove this call as we dont need to access the values more than once
    // let mods = extract_all_mods_metadata(rustique_options)?.values().collect::<Vec<&ModInfo>>();
    let mut table = Table::new();
    // table.set_header(vec!["Name", "ModID", "Version", "Description", "Website"]);
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
        .add_cell(Cell::new("Description").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("Website").add_attribute(Attribute::Bold).fg(Color::Blue));

    table.add_row(header);



    let sync_data_unwrapped = sync_data.ok();

    extract_all_mods_metadata(rustique_options)?
        .values()
        .for_each(|mod_info| {
            let mut row = Row::new();
            row.add_cell(Cell::new(&mod_info.name))
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
                }
            }

            row.add_cell(Cell::new(
                mod_info.description.as_ref().unwrap_or(&"".to_string()),
            ))
            .add_cell(Cell::new(
                mod_info.website.as_ref().unwrap_or(&"".to_string()),
            ));

            table.add_row(row);
        });

    println!("{}", table);

    Ok(())
}
