use crate::aliases::{ModFileName, ModID};
use crate::api::api_structs::{ModInfo};
use crate::commands::sync::{get_sync_data, ModSyncInfo};
use crate::config_manager::{get_config};
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_all_mods_metadata, gather_dependencies, gather_missing_dependencies, sanitize_string};
use crate::version_management::parse_version;
use owo_colors::OwoColorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Row, Table};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;
use crate::config_structs::{CellAttr, CellColor, ListColumn};
use crate::information_utils::prep_cell;
use crate::install_manager::Install;

fn grab_this_mod_deps(mod_info: &ModInfo, dep_list: &[Install]) -> String {
    let mut res = dep_list.iter()
        .filter(|i| mod_info.dependencies.as_ref().is_some_and(|deps| deps.contains_key(&i.mod_id)))
        .map(|i| i.mod_id.clone()).collect::<Vec<ModID>>();
    res.sort();
    res.dedup_by(|a,b|a.to_lowercase().eq(&b.to_lowercase()));
    res.join(",")
}

#[allow(clippy::filter_map_next)]
pub async fn new_list(mod_dir: &PathBuf, only_updated: bool) -> Result<(), RustiqueError> {
    let start_time = Instant::now();
    let config = get_config().read().await;

    let list_columns = &config.table.list;

    // setup headers
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let header_cells : Vec<Cell> = list_columns.headers.iter().map(|(column, properties)| {
        let color = properties.color.clone();
        let attr = properties.attribute.clone();

        let col_txt = match ListColumn::from_str(column) {
            Ok(ListColumn::Name) => "Name",
            Ok(ListColumn::ModId) => "ModID",
            Ok(ListColumn::Version) => "Version",
            Ok(ListColumn::GameVersion) => "Game Version",
            Ok(ListColumn::LatestVersion) => "Latest Version",
            Ok(ListColumn::PinnedVersion) => "Pinned Version",
            Ok(ListColumn::Description) => "Description",
            Ok(ListColumn::Deps) => "Dependencies",
            Ok(ListColumn::MissingDeps) => "Missing Dependencies",
            Ok(ListColumn::Changelog) => "Changelog",
            Ok(ListColumn::Filename) => "Filename",
            Ok(ListColumn::HasBackup) => "Has Backup",
            Ok(ListColumn::LastUpdateLocal) => "Last Update (Local)",
            Ok(ListColumn::LastUpdateRemote) => "Last Update (Remote)",
            Ok(ListColumn::Website) => "Website",
            _ => "N/A"
        };
        prep_cell(col_txt, color, attr, None, None)
    }).collect();
    table.set_header(Row::from(header_cells));

    // Unfortunately we need all this data to get accurate information for list
    let sync_data = get_sync_data(mod_dir).await?;
    let installed_mods = extract_all_mods_metadata(mod_dir).await?;
    
    let mut sorted_mods: Vec<(ModFileName, ModInfo)> = installed_mods.clone().into_iter().collect();
    sorted_mods.sort_by(|a,b| a.1.name.cmp(&b.1.name));
    
    let all_deps = gather_dependencies(&installed_mods);
    let missing_deps = gather_missing_dependencies(&installed_mods, &[], &sync_data.rustique_sync);

    // iterate over all_modinfo and fill the table with what is needed

    let rows: Vec<Row> = sorted_mods
        .iter()
        .filter(|(_, mod_info)| {
            if only_updated {
                if let Some(mod_sync) = sync_data.rustique_sync.iter().find_map(|(_, mod_sync_info)| {
                    if mod_sync_info.mod_name == mod_info.name {
                        Some(mod_sync_info.clone())
                    } else {
                        None
                    }
                }) {
                    mod_sync.latest_known_version != mod_sync.installed_version
                } else {
                    false
                }
            } else {
                true
            }
        })
        .map(|(filename, mod_info)| {
           
            let pkg = config.pkg.iter().find(|p| p.mod_id.eq(&mod_info.mod_id));
            let cells: Vec<Cell> = list_columns.cells.iter().map(|(column, properties)| { 
                let color = properties.color.clone();
                let attr = properties.attribute.clone();

                let (mod_sync_id, mod_sync_data): (ModID, ModSyncInfo) = sync_data.rustique_sync
                    .iter()
                    .filter_map(|(mod_id, mod_sync)| {
                    if **mod_id == mod_info.mod_id
                        || mod_info.name == mod_sync.mod_name
                        || *filename == mod_sync.file_name {
                        Some((mod_id.clone(), mod_sync.clone()))
                    } else {
                        None
                    }
                }).next().unwrap_or_default();

                match <ListColumn as FromStr>::from_str(column) {
                    Ok(ListColumn::Name) => {
                        prep_cell(&mod_info.name.clone(), color, attr, None, None)

                    },
                    Ok(ListColumn::ModId) => {
                        let txt = if !mod_info.mod_id.is_empty() {
                            mod_info.mod_id.clone()
                        } else if !mod_sync_id.is_empty() {
                            mod_sync_id.clone()
                        } else {
                            String::from("UNKNOWN")
                        };
                        prep_cell(&txt, color, attr, None, None)
                    },
                    Ok(ListColumn::Version) => {
                        let txt = parse_version(&mod_info.version.clone().unwrap_or_default()).unwrap();
                        prep_cell(&txt.to_string(), color, attr, None, Some(CellAlignment::Right))
                    },
                    Ok(ListColumn::LatestVersion) => {
                        let latest = mod_sync_data.latest_known_version.clone();
                        let mut pinned = String::new(); 
                        if pkg.is_some() {
                            pinned += " (pinned)";
                        }
                        
                        if latest == mod_info.version.clone().unwrap_or(String::new()) {
                            prep_cell((latest + &pinned).as_str(), color, attr, None, Some(CellAlignment::Right))
                        } else {
                            prep_cell((latest + &pinned).as_str(), Some(CellColor::Red), Some(CellAttr::Bold), None, Some(CellAlignment::Right))
                        }

                    },
                    Ok(ListColumn::PinnedVersion) => {
                        pkg.and_then(|mod_pkg| mod_pkg.pinned_version.as_ref())
                            .map_or_else(
                                || prep_cell("", color.clone(), attr.clone(), None, Some(CellAlignment::Right)),
                                |pinned_version| prep_cell(pinned_version, color.clone(), attr.clone(), None, Some(CellAlignment::Right))
                            )
                    }
                    Ok(ListColumn::Description) => {
                        let txt = sanitize_string(&mod_info.description.clone().unwrap_or(String::new()));
                        prep_cell(&txt, color, attr, None, None)
                    },
                    Ok(ListColumn::Deps) => {
                        let deps = grab_this_mod_deps(mod_info, &all_deps.clone());
                        prep_cell(&deps, color, attr, Some(','), None)
                    }
                    Ok(ListColumn::MissingDeps) => {
                       let missing = grab_this_mod_deps(mod_info, &missing_deps.clone());
                        prep_cell(&missing, color, attr, Some(','), None)
                    }
                    Ok(ListColumn::Filename) => {
                        prep_cell(filename.as_str(), color, attr, None, None)
                    },
                    
                    Ok(ListColumn::GameVersion) => {
                        // show a range from oldest to newest, newest is always the first value of the list
                        let gv  = mod_sync_data.game_versions;
                        let game_versions = if gv.len() > 1  {
                            format!("{} - {}", gv[gv.len() - 1], gv[0])
                        } else {
                            gv.join(",")
                        };
                        prep_cell(&game_versions, color, attr, None, Some(CellAlignment::Right))
                    }
                    Ok(ListColumn::LastUpdateLocal 
                       | ListColumn::LastUpdateRemote 
                       | ListColumn::HasBackup 
                       | ListColumn::Changelog) => {
                        prep_cell("NOT IMPLEMENTED", color, attr, None, None)
                    }
                    Ok(ListColumn::Website) => {
                        prep_cell(mod_info.website.clone().unwrap_or_default().as_str(), color, attr, None, None)
                    },
                    _ => prep_cell("", color, attr, None, None)
                } 
            }).collect();

        Row::from(cells)
    }).collect();

    table.add_rows(rows);

    println!("{table}");
    print!("{} {}", "Total Mod Count:".bright_green().bold().on_black(), installed_mods.len().to_string().bright_purple().on_black());

     if config.show_execution_time {
        let elapsed = format!("{:.2}", start_time.elapsed().as_secs_f64());
        println!(" - {}: {}{}","List operation took".bright_green().bold().on_black(), elapsed.bright_purple().on_black(), "s".bright_yellow().on_black());
    } else {
        println!();
    }

    Ok(())
}
