use std::fmt::Display;
use std::process::exit;
use clap::{ValueEnum};
use comfy_table::{Attribute, CellAlignment, Color};
use tracing::{error};
use crate::commands::arg_structs::config_table_args::{ResetArgs, TableArgs, TableArgsSubCommands, TableGroup, TableSubCommands, TableSubFlags};
use crate::config::config_manager::get_config;
use crate::config::config_structs::{CellAttr, CellColor, ColumnProperties, TableSection, Tables};
use crate::config::flatten_map::FlattenMap;
use crate::information_utils::{display_table, notice, CellData};

pub async fn config_table(args: &TableArgs) {

    let config = &mut get_config().write().await;
    let table = &mut config.table;

    let commands = &args.subcommand;

    match commands {
        TableSubCommands::Set(cmd) => {
            match &cmd.subcommand {
                TableArgsSubCommands::List(arg) => {
                    modify_table(arg, &mut table.list, false);
                }
                TableArgsSubCommands::Search(arg) => {
                    modify_table(arg, &mut table.search, false);
                }
            }
        },
        TableSubCommands::Del(cmd) => {
            match &cmd.subcommand {
                TableArgsSubCommands::List(arg) => {
                    modify_table(arg, &mut table.list, true);
                }
                TableArgsSubCommands::Search(arg) => {
                    modify_table(arg, &mut table.search, true);
                }
            }
        },
        TableSubCommands::List => {
            let list = table.list.clone(); // create a readonly clone to use
            let search = table.search.clone();

            let mut list_headers_vec: Vec<(CellData, CellData)> = Vec::new();
            fill_vec_from_section(&list.headers, &mut list_headers_vec);
            
            let mut list_cells_vec: Vec<(CellData, CellData)> = Vec::new();
            fill_vec_from_section(&list.cells, &mut list_cells_vec);
            
            let mut search_headers_vec: Vec<(CellData, CellData)> = Vec::new();
            fill_vec_from_section(&search.headers, &mut search_headers_vec);
            
            let mut search_cells_vec: Vec<(CellData, CellData)> = Vec::new();
            fill_vec_from_section(&search.cells, &mut search_cells_vec);
            
            notice("List Table Headers", Some(Color::Yellow), vec![Attribute::Bold]);
            display_table(list_headers_vec, None);
            
            notice("List Table Cells", Some(Color::Yellow), vec![Attribute::Bold]); 
            display_table(list_cells_vec, None);
            
            notice("Search Table Headers", Some(Color::Yellow), vec![Attribute::Bold]); 
            display_table(search_headers_vec, None);
            
            notice("Search Table Cells", Some(Color::Yellow), vec![Attribute::Bold]); 
            display_table(search_cells_vec, None);
        },
        TableSubCommands::Reset(args) => {
            match args.command {
                ResetArgs::Search => {
                    reset_table(&mut table.search, "search");
                }
                ResetArgs::List => {
                    reset_table(&mut table.list, "list");
                }
            }
        }
    }

    match config.save(None) {
        Ok(()) => {}
        Err(e) => {
            error!("Unable to save config table {}", e.to_string());
        }
    }
}

fn fill_vec_from_section(section: &FlattenMap, the_vec: &mut Vec<(CellData, CellData)>) {
    for (name, property) in section.iter() {
        let color = property.color.clone().unwrap_or(CellColor::Green);
        let attr = property.attribute.clone().unwrap_or(CellAttr::NoHidden);
                
        let c_txt = CellData::new(format!("{name}.color"), Some(Color::Green), vec![], Some(CellAlignment::Left));
        let c_val = CellData::new(color.to_string(), Some(Color::from(color.clone())), vec![], Some(CellAlignment::Left));
        let a_txt = CellData::new(format!("{name}.attribute"), Some(Color::Blue), vec![], Some(CellAlignment::Left));
        let a_val = CellData::new(attr.to_string(), Some(Color::from(color.clone())), vec![Attribute::from(attr.clone())], Some(CellAlignment::Left));
        the_vec.extend(vec![(c_txt, c_val), (a_txt, a_val)]); 
    }
}

fn reset_table(section: &mut TableSection, table_type: &str) {
    match table_type {
        "list" => section.clone_from(&Tables::list_defaults()),
        "search" => section.clone_from(&Tables::search_defaults()),
        _ => {}
    }
}
fn modify_table<T>(arg: &TableSubFlags<T>, section: &mut TableSection, delete: bool)
where T: ValueEnum + Clone + Send + Sync + Display + 'static  {
    let group = cell_or_header(&arg.group);
    let mut fields: Vec<T> = vec![];
    if arg.field.is_some() {
        let field = arg.field.as_ref().unwrap_or_else(|| panic!("Invalid field type"));
        fields.push(field.clone());
    } else if !arg.fields.is_empty() {
        fields.clone_from(&arg.fields);
    } else {
        panic!("Invalid field type");

    }

    let (main, other) = match group.as_str() {
        "headers" => (&mut section.headers, &mut section.cells),
        "cells" => (&mut section.cells, &mut section.headers),
        _ => panic!("Invalid group"),
    };

    let color = &arg.color;
    let attr = &arg.attr;

    for field in &fields {
        if delete {
            main.shift_remove_entry(&field.to_string());
            other.shift_remove_entry(&field.to_string());
        } else {
            main.entry(field.to_string()).and_modify(|f| {
                if color.is_some() {
                    f.color.clone_from(color);
                }
                if attr.is_some() {
                    f.attribute.clone_from(attr);
                }
            }).or_insert(ColumnProperties {
                color: color.clone(),
                attribute: attr.clone(),
            });

            // Enable the field for both, but only set the values of what was actually passed
            other.entry(field.to_string()).or_default();
        }

    }
}

fn cell_or_header(arg: &TableGroup) -> String {
    match (arg.cells, arg.headers) {
        (true, _) => "cells".to_string(),
        (_, true) => "headers".to_string(),
        _ => {
            error!("You must select either --cells or --headers");
            exit(1)
        }
    }
}