use crate::commands::arg_structs::config_args::{BoolArgs, CommonArgs, ConfigCommand, ConfigSubCommand};
use crate::utils::{get_expanded_path, parse_json_file};
use std::path::PathBuf;
use std::process::exit;
use comfy_table::{Attribute, CellAlignment, Color, ContentArrangement, Row, Table};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_FULL_CONDENSED};
use tracing::{warn};
use crate::config::config_table::config_table;
use crate::commands::sync::{GameVersionSync};
use crate::config::config_manager::{get_config, Config, Package};
use crate::config::config_structs::{CellAttr, CellColor};
use crate::consts::FILE_GAME_VERSION_SYNC;
use crate::information_utils::{command_output, display_table, notice, prep_cell, CellData};
use crate::traits::string_ext::StrLowerExt;
use crate::version_management::parse_version;

pub async fn parse_config_args(config_cmd: &ConfigCommand) {
    match &config_cmd.subcommand {
        ConfigSubCommand::Set(args) => {
            set(&args.common).await;
        },
        ConfigSubCommand::List => {
            list().await;
        },
        ConfigSubCommand::Del(args) => {
            del(args).await;
        },
        ConfigSubCommand::Table(args) => {
            config_table(args).await;
        }
    }
}

async fn set(args: &CommonArgs) {

    let mut config = get_config().write().await;

    let mut display_vec: Vec<(CellData, CellData)> = Vec::new();

    let mut save = false;

    if let Some(path) = &args.mods_dir {
        let dir = get_expanded_path(PathBuf::from(path));
        if dir.exists() {
            config.mod_dir = dir.to_string_lossy().to_string();
            save = true;

           display_vec.push(command_output("config.mods_dir".to_string(), path.to_string()));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }

    if let Some(notif) = &args.notify_of_unzipped_mods {
        config.notify_of_unzipped_mods = *notif;
        save = true;
        display_vec.push(command_output("config.show_mod_dir_warning".to_string(), notif.to_string()));
    }

    if let Some(version) = &args.pin_game_version {

        let gv_sync_file = Config::get_path().join(FILE_GAME_VERSION_SYNC);
        let mut game_versions = match parse_json_file::<GameVersionSync>(&gv_sync_file) {
            Ok(game_versions) => game_versions,
            Err(err) => {
                warn!("{}", err);
                exit(1);
            }
        };

        game_versions.game_versions.sort_by(|v1, v2| {
            let v1_version = parse_version(v1).unwrap();
            let v2_version = parse_version(v2).unwrap();
            v1_version.cmp(&v2_version)
        });

        game_versions.game_versions.reverse();

        let v = if version.lower_contains("v") {
            version.to_string()
        } else {
            "v".to_owned()+version
        };

        if game_versions.game_versions.contains(&v) {
            config.pinned_game_version.clone_from(&v);
            save = true;
            display_vec.push(command_output("config.pinned_game_version".to_string(), v));
        } else {
            notice("Invalid game version. The version must be one of the following: ", Some(Color::Yellow), vec![Attribute::Bold]);
            notice(&format!("[{}]",game_versions.game_versions.join("], [").as_str()), Some(Color::Magenta), vec![]);
        }
    }
    
    if let Some(with_mod) = &args.with_mod {
        if let Some(version) = &args.pin_version {
            let mut found = false;
            for package in &mut config.pkg {
                if package.mod_id.eq_ignore_ascii_case(with_mod) {
                    package.pinned_version = Some(version.clone());
                    found = true;
                    break;
                }
            }
            
            if !found {
                config.pkg.push(Package {
                    mod_id: with_mod.clone(),
                    pinned_version: Some(version.clone())
                });
            }
            save = true;
            display_vec.push(command_output(format!("Pinned: {with_mod}"), version.to_string()));
        }
    }

    if let Some(val) = &args.show_execution_time {

        config.show_execution_time = *val;
        save = true;

        display_vec.push(command_output("config.show_execution_time".to_string(), val.to_string()));
    }

    if let Some(zip_it) = &args.zip_mod_dirs {

        config.zip_mod_files = *zip_it;
        save = true;

        display_vec.push(command_output("config.zip_mod_files".to_string(), zip_it.to_string()));
    }

    if let Some(backup) = &args.backup_mods {
        config.backup_mods = *backup;
        save = true;

        display_vec.push(command_output("config.backup_mods".to_string(), backup.to_string()));
    }

    if let Some(backup_dir) = &args.backup_mods_dir {
        let dir = get_expanded_path(PathBuf::from(backup_dir));
        if dir.exists() {
            config.backup_mods_dir = dir.to_string_lossy().to_string();
            save = true;

            display_vec.push(command_output("config.backup_mods_dir".to_string(), dir.to_string_lossy().to_string()));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }
    
    if let Some(download_dir) = &args.game_download_dir {
        let dir = get_expanded_path(PathBuf::from(download_dir));
        if dir.exists() {
            config.game_download_dir = dir.to_string_lossy().to_string();
            save = true;
            
            display_vec.push(command_output("config.game_download_dir".to_string(), download_dir.to_string()));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }

    if save {
        config.save(None).unwrap();
        display_table(display_vec, None);
    }


}


async fn del(args: &BoolArgs) {

    let mut config = get_config().write().await;
    let defaults = Config::default();
    let mut display_vec: Vec<(CellData, CellData)> = Vec::new();

    let mut save = false;

    if args.backup_mods_dir {
        config.backup_mods_dir.clone_from(&defaults.backup_mods_dir);
        save = true;
        display_vec.push(command_output("config.backup_mods_dir", defaults.backup_mods_dir));
    }

    if args.zip_mod_dirs {
        config.zip_mod_files = defaults.zip_mod_files;
        save = true;
        display_vec.push(command_output("config.zip_mod_files", defaults.zip_mod_files.to_string()));
    }

    if args.backup_mods {
        config.backup_mods = defaults.backup_mods;
        save = true;
        display_vec.push(command_output("config.backup_mods", defaults.backup_mods.to_string()));
    }

    if args.pin_game_version {
        config.pinned_game_version.clone_from(&defaults.pinned_game_version);
        save = true;
        display_vec.push(command_output("config.pinned_game_version", defaults.pinned_game_version));
    }

    if args.mod_dir {
        config.mod_dir.clone_from(&defaults.mod_dir);
        save = true;
        display_vec.push(command_output("config.mod_dir", defaults.mod_dir));
    }

    if args.notify_of_unzipped_mods {
        config.notify_of_unzipped_mods = defaults.notify_of_unzipped_mods;
        save = true;
        display_vec.push(command_output("config.notify_of_unzipped_mods", config.notify_of_unzipped_mods.to_string()));
    }

    if args.pinned_mod.is_some() {
        let Some(mod_id) = &args.pinned_mod else {
                warn!("You must provide a Mod ID before anything can be removed. Run [./Rustique config list] to show all valid options");
                exit(1);
            };

        if !config.pkg.is_empty() {
            config.pkg.retain(|p| p.mod_id != *mod_id);
            save = true;
            display_vec.push(command_output("Removed pinned version from: ", mod_id));
        }
    }
    
    if args.game_download_dir {
        config.game_download_dir.clone_from(&defaults.game_download_dir);
        save = true;
        display_vec.push(command_output("config.game_download_dir", defaults.game_download_dir));
    }


    if save {
        display_table(display_vec, None);
        config.save(None).unwrap();
    }
}

async fn list() {
    let config = get_config().read().await;
    let display_vec: Vec<(CellData, CellData)> = vec![
        command_output("config.mod_dir",                 config.mod_dir.to_string()),
        command_output("config.backup_mods_dir",         config.backup_mods_dir.to_string()),
        command_output("config.game_download_dir",       config.game_download_dir.to_string()),
        command_output("config.backup_mods",             config.backup_mods.to_string()),
        command_output("config.zip_mod_files",           config.zip_mod_files.to_string()),
        command_output("config.show_execution_time",     config.show_execution_time.to_string()),
        command_output("config.notify_of_unzipped_mods", config.notify_of_unzipped_mods.to_string()),
        command_output("config.pinned_game_version",     config.pinned_game_version.to_string()),
    ];
    
    display_table(display_vec, None);

    if !config.pkg.is_empty() {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL_CONDENSED).apply_modifier(UTF8_ROUND_CORNERS).set_content_arrangement(ContentArrangement::Dynamic);
        let headers = vec![
            prep_cell("Mod ID", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
            prep_cell("Pinned Version", Some(CellColor::Green), Some(CellAttr::Bold), None, Some(CellAlignment::Right)),
        ];
        table.set_header(Row::from(headers));


        let mut rows: Vec<Row> = Vec::with_capacity(config.pkg.len());
        for pkg in &config.pkg {
            let mod_name = prep_cell(&pkg.mod_id.clone(), Some(CellColor::Yellow), None, None, None);
            let pinned_version = prep_cell(&pkg.pinned_version.clone().unwrap_or(String::new()), Some(CellColor::Magenta), None, None, Some(CellAlignment::Right));
            rows.push(Row::from(vec![mod_name, pinned_version]));
        }

        table.add_rows(rows);
        println!("{table}");
    }
}