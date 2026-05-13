use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use rustique_core::aliases::{ModFileName, ModID};
use rustique_core::api::api_structs::{ModInfo};
use rustique_core::rustique_errors::RustiqueError;
use rustique_core::utils::{extract_all_mods_metadata, gather_dependencies, gather_missing_dependencies, split_modid_version, sanitize_string, format_for_csv, html_parse};
use rustique_core::version_management::parse_version;
use owo_colors::OwoColorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Row, Table};
use std::str::FromStr;
use std::time::Instant;
use csv::Writer;
use tracing::{debug, info};
use crate::commands::arg_structs::list_args::ListExport;
use rustique_core::config::config_manager::get_config;
use rustique_core::config::config_structs::{CellAttr, CellColor, ListColumn, TableSection};
use rustique_core::information_utils::prep_cell;
use rustique_core::install_manager::Install;
use rustique_core::sync_structs::ModSyncInfo;
use rustique_core::traits::ref_ext::PathRef;
use crate::commands::sync::{get_sync_data, sync};

fn grab_this_mod_deps(mod_info: &ModInfo, dep_list: &[Install]) -> String {
    let mut res = dep_list.iter()
        .filter(|i| mod_info.dependencies.contains_key(&i.mod_id))
        .map(|i| String::from("[")+i.mod_id.clone().as_str() + "@" + i.version_to_install.clone().as_str() + "]").collect::<Vec<String>>();
    res.sort();
    res.dedup_by(|a,b|a.to_lowercase().eq(&b.to_lowercase()));
    res.join(", ")
}

#[allow(clippy::filter_map_next, clippy::too_many_lines, clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub async fn cmd_list(
    mod_dir: impl PathRef, 
    only_updated: bool,
    only_pinned: bool,
    modpack_call: bool, 
    local_mp_call: bool, 
    columns: Vec<ListColumn>, 
    export: Option<ListExport>, 
    write_file: Option<PathBuf>
) -> Result<(), RustiqueError> {
    
    let mod_dir = mod_dir.as_ref();
    let start_time = Instant::now();
    let config = get_config().read().await;

    let config_columns = &config.table.list;


    // setup headers
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    // if fields set the columns from that
    // else get columns from config

    let mut override_columns = TableSection::new();

    let list_columns = if columns.is_empty() {
        config_columns
    } else {

        for col in columns {
            override_columns.headers.with(col.as_str(), None, None);
            override_columns.cells.with(col.as_str(), None,None);
        }

        &override_columns
    };


    let header_cells : Vec<Cell> = list_columns.headers.iter().filter_map(|(column, properties)| {
        let color = properties.color.clone();
        let attr = properties.attribute.clone();

        let col_txt = match ListColumn::from_str(column) {
            Ok(ListColumn::Name)            => "Name",
            Ok(ListColumn::ModId)           => "ModID",
            Ok(ListColumn::Version)         => "Version",
            Ok(ListColumn::GameVersion)     => "Game Version",
            Ok(ListColumn::LatestVersion)   => "Latest Version",
            Ok(ListColumn::PinnedVersion)   => "Pinned Version",
            Ok(ListColumn::Description)     => "Description",
            Ok(ListColumn::Deps)            => "Dependencies",
            Ok(ListColumn::MissingDeps)     => "Missing Dependencies",
            Ok(ListColumn::Changelog)       => "Changelog",
            Ok(ListColumn::Filename)        => "Filename",
            Ok(ListColumn::HasBackup)       => "Has Backup",
            Ok(ListColumn::LastUpdateLocal) => "Last Update (Local)",
            Ok(ListColumn::LastUpdateRemote)    => "Last Update (Remote)",
            Ok(ListColumn::Website)             => "Website",
            Ok(ListColumn::ModURL)              => "Mod URL",
            _ => "N/A"
        };


        if (local_mp_call || modpack_call) && matches!(ListColumn::from_str(column), Ok(ListColumn::MissingDeps)) {
            debug!("modpack_call on missingDeps");
            return None;
        }
        
        if local_mp_call && matches!(ListColumn::from_str(column), Ok(ListColumn::LatestVersion)) {
            debug!("local modpack_call on LatestVersion");
            return None; 
        }
        
        Some(prep_cell(col_txt, color, attr, None, None))
    }).collect();
    table.set_header(Row::from(header_cells));

    // Unfortunately we need all this data to get accurate information for list
    let sync_data = if local_mp_call {
        None
    } else {
        Some(match get_sync_data(mod_dir, false).await {
            Ok(s) => s,
            Err(_) => {
                sync(mod_dir, false, vec![]).await
            }?
        })
    };
    
    let installed_mods = extract_all_mods_metadata(mod_dir, false).await?;
    
    let mut sorted_mods: Vec<(ModFileName, ModInfo)> = installed_mods.clone().into_iter().collect();
    sorted_mods.sort_by(|a,b| a.1.name.cmp(&b.1.name));
    
    let all_deps = gather_dependencies(&installed_mods);
    
    // If modpack local is called, we ignore sync data for the mod_dir as they will only be local modpacks, there is no sync data
    // So we set to an empty hashmap 
    let sync_hashmap = if let Some(sd) = &sync_data {
        &sd.rustique_sync
    } else {
        &BTreeMap::new()
    };
    
    let missing_deps = gather_missing_dependencies(&installed_mods, &[], sync_hashmap);

    let mut enabled_modpacks: HashMap<ModID, Vec<ModID>> = config.modpacks.enabled.iter().map(|m| (m.clone(), Vec::new())).collect();


    for (pack_id, v) in &mut enabled_modpacks {
        let (pack_id, _) = split_modid_version(pack_id);
        let mpath = Path::new(&config.modpacks.modpack_dir).join("installed").join(pack_id);
        if mpath.exists() {
            let mp_sync_file = match get_sync_data(&mpath, false).await {
                Ok(s) => {
                    info!("Sync data found");
                    s
                },
                Err(e) => { 
                    info!("{}: {}", "ERROR: ".red().bold(), e);
                    sync(mod_dir, true, vec![]).await? 
                }
            };
            
            let keys: Vec<ModID> = mp_sync_file.rustique_sync.into_keys().map(|k| split_modid_version(k).0).collect();
            v.extend(keys);
        }
    }
    

    // iterate over all_modinfo and fill the table with what is needed

    let rows: Vec<Row> = sorted_mods
        .iter()
        .filter(|(_, mod_info)| {
            // Show all mods if local_mp_call is true
            // OR
            // show all mods if only_updated is false
            // OR
            // show only updates if only_updated is true, and local_mp_call is false
            local_mp_call || !only_updated
                || sync_hashmap.values()
                .find(|sync| sync.mod_name == mod_info.name)
                .is_some_and(|sync| sync.latest_known_version != sync.installed_version)
        })
        .filter_map(|(filename, mod_info)| {
            
            let file_is_symlink = mod_dir.join(filename).is_symlink();
           
            let pkg = config.pkg.iter().find(|p| p.mod_id.eq(&mod_info.mod_id));

            if only_pinned && pkg.is_none() {
                return None
            }
            
            let cells: Vec<Cell> = list_columns.cells.iter().filter_map(|(column, properties)| {
                
                let color = properties.color.clone();
                let attr = properties.attribute.clone();

                let (mod_sync_id, mod_sync_data): (ModID, ModSyncInfo) = sync_hashmap
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
                        Some(prep_cell(&mod_info.name, color, attr, None, None))
                    },
                    Ok(ListColumn::ModId) => {
                        
                        let (mut txt, mut the_color) = (String::new(), color);
                        
                        let mid = if !mod_info.mod_id.is_empty() {
                            mod_info.mod_id.clone().to_lowercase()
                        } else if !mod_sync_id.is_empty() {
                            mod_sync_id.clone().to_lowercase()
                        } else {
                            String::from("UNKNOWN")
                        };
                        
                        if modpack_call && config.modpacks.enabled.contains(&mid) {
                            txt += "(enabled) ";
                            the_color = Some(CellColor::Green);
                        }
                        
                        if file_is_symlink {
                            txt += "(";
                            txt += enabled_modpacks.iter()
                                .find(|(_,v)| v.contains(&mid))
                                .map_or("?modpack?",|(k,_)| k);
                            
                            txt += ") ";
                            the_color = Some(CellColor::DarkYellow);
                        }
                        
                        Some(prep_cell(txt + &mid, the_color, attr, None, None))
                    },
                    Ok(ListColumn::Version) => {
                        let txt = parse_version(&mod_info.version.clone().unwrap_or_default()).unwrap().to_string();
                        Some(prep_cell(txt.to_string(), color, attr, None, Some(CellAlignment::Right)))
                    },
                    Ok(ListColumn::LatestVersion) => {
                        // No need to show LatestVersion for local modpack, they are always the latest version
                        if local_mp_call {
                            return None    
                        }
                        
                        let latest = mod_sync_data.latest_known_version.clone();
                        let mut pinned = String::new(); 
                        if pkg.is_some() {
                            pinned += " (pinned)";
                        }
                        
                        if latest == mod_info.version.clone().unwrap_or(String::new()) {
                            Some(prep_cell((latest + &pinned).as_str(), color, attr, None, Some(CellAlignment::Right)))
                        } else {
                            Some(prep_cell((latest + &pinned).as_str(), Some(CellColor::Red), Some(CellAttr::Bold), None, Some(CellAlignment::Right)))
                        }

                    },
                    Ok(ListColumn::PinnedVersion) => {
                        Some(pkg.and_then(|mod_pkg| mod_pkg.pinned_version.as_ref())
                            .map_or_else(
                                || prep_cell("", color.clone(), attr.clone(), None, Some(CellAlignment::Right)),
                                |pinned_version| prep_cell(pinned_version, color.clone(), attr.clone(), None, Some(CellAlignment::Right))
                            ))
                    }
                    Ok(ListColumn::Description) => {
                        let mut txt = sanitize_string(&mod_info.description.clone().unwrap_or(String::new()));

                        if let Some(out) = &export {
                            if matches!(out, ListExport::Csv) {
                                txt = format_for_csv(txt);
                            }
                        }

                        Some(prep_cell(&txt, color, attr, None, None))
                    },
                    Ok(ListColumn::Deps) => {
                        let mut deps = grab_this_mod_deps(mod_info, &all_deps.clone());
                        let mut table_sep = Some(',');
                        if let Some(out) = &export {
                           if matches!(out, ListExport::Csv) {
                               deps = format_for_csv(deps);
                               table_sep = None;
                           }
                        }

                        Some(prep_cell(&deps, color, attr, table_sep, None))
                    }
                    Ok(ListColumn::MissingDeps) => {
                        if modpack_call {
                            return None;
                        }
                        let mut missing = grab_this_mod_deps(mod_info, &missing_deps.clone());
                        let mut table_sep = Some(',');
                        // if output === csv, replace all , with space
                        if let Some(out) = &export {
                           if matches!(out, ListExport::Csv) {
                               missing = format_for_csv(missing);
                               table_sep = None;
                           }
                        }
                        
                        Some(prep_cell(&missing, color, attr, table_sep, None))
                    }
                    Ok(ListColumn::Filename) => {
                        Some(prep_cell(filename.as_str(), color, attr, None, None))
                    },
                    
                    Ok(ListColumn::GameVersion) => {
                        // show a range from oldest to newest, newest is always the first value of the list
                        let gv  = mod_sync_data.game_versions;
                        let game_versions = if gv.len() > 1  {
                            format!("{} - {}", gv[gv.len() - 1], gv[0])
                        } else if let Some(out) = &export {
                            if matches!(out, ListExport::Csv) {
                                gv.join(" ")
                            } else {
                                gv.join(",")
                            }
                        } else {
                            gv.join(",")
                        };
                        Some(prep_cell(&game_versions, color, attr, None, Some(CellAlignment::Right)))
                    }
                    Ok(ListColumn::LastUpdateLocal 
                       | ListColumn::LastUpdateRemote 
                       | ListColumn::HasBackup) => {
                        Some(prep_cell("NOT IMPLEMENTED", color, attr, None, None))
                    }
                    Ok(ListColumn::Changelog) => {
                        // let mut changelog = sanitize_string(&mod_sync_data.latest_changelog);
                        let mut changelog = &mod_sync_data.latest_changelog;

                        let out =  html_parse(&mut changelog, usize::from(table.width().unwrap_or_default())).unwrap_or_default();
                        let changelog = if let Some(output) = &export {
                            if matches!(output, ListExport::Csv) {
                                format_for_csv(out)
                            } else {
                                out
                            }
                        } else {
                           out
                        };
                        
                        Some(prep_cell(changelog, color, attr, None, None))
                    }
                    Ok(ListColumn::Website) => {
                        Some(prep_cell(mod_info.website.clone().unwrap_or_default().as_str(), color, attr, None, None))
                    },
                    Ok(ListColumn::ModURL) => {
                        let url = format!("https://mods.vintagestory.at/show/mod/{}", mod_sync_data.asset_id);

                        Some(prep_cell(url, color, attr, None, None))
                    }
                    _ => Some(prep_cell("", color, attr, None, None))
                } 
            }).collect();



            Option::from(Row::from(cells))
    }).collect();


    table.add_rows(rows);
    
    if let Some(out) = &export {
        if matches!(out, ListExport::Csv) {
            
           
            let writer: Box<dyn Write> = match write_file {
                Some(f) => Box::new(File::create(f)?),
                None => Box::new(stdout()),
            };
            
            let mut wtr = Writer::from_writer(writer);
            
            if let Some(row) = table.header() {
                write_record(&mut wtr, row.cell_iter().map(|c|c.content()))?;
            }

            for row in table.row_iter() {
                write_record(&mut wtr, row.cell_iter().map(|c|c.content()))?;
            }
            
           
            wtr.flush()?;
        }
    } else {

        println!("{table}");
        print!("{} {}", "Total Mod Count:".bright_green().bold().on_black(), installed_mods.len().to_string().bright_purple().on_black());

        if config.show_execution_time {
            let elapsed = format!("{:.2}", start_time.elapsed().as_secs_f64());
            println!(" - {}: {}{}","List operation took".bright_green().bold().on_black(), elapsed.bright_purple().on_black(), "s".bright_yellow().on_black());
        }
    }

    Ok(())
}

/// simple helper method for writer a record to the CSV writer. to avoid duplicate code
fn write_record<I, T>(writer: &mut Writer<Box<dyn Write>>, collection: I) -> Result<(),RustiqueError>
where 
    I: IntoIterator<Item=T>,
    T: AsRef<[u8]>
{
    writer.write_record(collection).map_err(|e| RustiqueError::SimpleError(e.to_string()))
}