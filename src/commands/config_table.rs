use std::fmt::Display;
use std::process::exit;
use clap::{arg, ValueEnum};
use tracing::{error, info};
use crate::commands::arg_structs::config_table_args::{TableArgs, TableArgsSubCommands, TableGroup, TableSubCommands, TableSubFlags};
use crate::config_manager::get_config;
use crate::config_structs::{ColumnProperties, ListColumn, TableSection};

pub async fn config_table(args: &TableArgs) {

    let config = &mut get_config().write().await;
    let table = &mut config.table;

    let commands = &args.subcommand;

    match commands {
        TableSubCommands::Set(cmd) => {
            match &cmd.subcommand {
                TableArgsSubCommands::List(arg) => {
                    modify_table(arg, &mut table.list);
                }
                TableArgsSubCommands::Search(arg) => {
                    modify_table(arg, &mut table.search);
                }
            }
        },
        TableSubCommands::Del(cmd) => {

        },
        TableSubCommands::List => {

        }
    }
    
    config.save(None).unwrap();
}

fn modify_table<T>(arg: &TableSubFlags<T>, section: &mut TableSection) 
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

    let ch = match group.as_str() {
        "headers" => &mut section.headers,
        "cells" => &mut section.cells,
        _ => panic!("Invalid group"),
    };
    
    let color = &arg.color;
    let attr = &arg.attr;

    for field in &fields {
        ch.entry(field.to_string()).and_modify(|f| {
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