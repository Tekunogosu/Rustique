use crate::commands::arg_structs::config_args::{BoolArgs, CommonArgs, ConfigCommand, ConfigSubCommand};
use crate::config_manager::{get_config, Config};
use crate::utils::get_expanded_path;
use std::path::PathBuf;
use tracing::{warn};
use crate::commands::config_table::config_table;
use crate::information_utils::{command_output, display_table, CellData};

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

    if let Some(path) = &args.mods_dir {
        let dir = get_expanded_path(PathBuf::from(path));
        if dir.exists() {
            config.mod_dir = dir.to_string_lossy().to_string();
            config.save(None).unwrap();

           display_vec.push(command_output("config.mods_dir".to_string(), path.to_string()));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }

    if let Some(notif) = &args.notify_of_unzipped_mods {
        config.notify_of_unzipped_mods = *notif;
        config.save(None).unwrap();

       display_vec.push(command_output("config.show_mod_dir_warning".to_string(), notif.to_string()));
    }

    if let Some(version) = &args.pin_game_version {
        config.pinned_game_version = version.to_string();
        config.save(None).unwrap();

        display_vec.push(command_output("config.pinned_game_version".to_string(), version.to_string()));
    }

    if let Some(val) = &args.show_execution_time {

        config.show_execution_time = *val;
        config.save(None).unwrap();

        display_vec.push(command_output("config.show_execution_time".to_string(), val.to_string()));
    }

    if let Some(zip_it) = &args.zip_mod_dirs {

        config.zip_mod_files = *zip_it;
        config.save(None).unwrap();

        display_vec.push(command_output("config.zip_mod_files".to_string(), zip_it.to_string()));
    }

    if let Some(backup) = &args.backup_mods {
        config.backup_mods = *backup;
        config.save(None).unwrap();

        display_vec.push(command_output("config.backup_mods".to_string(), backup.to_string()));
    }

    if let Some(backup_dir) = &args.backup_mods_dir {
        let dir = get_expanded_path(PathBuf::from(backup_dir));
        if dir.exists() {
            config.backup_mods_dir = dir.to_string_lossy().to_string();
            config.save(None).unwrap();

            display_vec.push(command_output("config.backup_mods_dir".to_string(), dir.to_string_lossy().to_string()));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }

    display_table(display_vec, None);

}


async fn del(args: &BoolArgs) {

    let mut config = get_config().write().await;
    let defaults = Config::default();
    let mut display_vec: Vec<(CellData, CellData)> = Vec::new();


    if args.backup_mods_dir {
        config.backup_mods_dir.clone_from(&defaults.backup_mods_dir);
        display_vec.push(command_output("config.backup_mods_dir".to_string(), defaults.backup_mods_dir.to_string()));
    }

    if args.zip_mod_dirs {
        config.zip_mod_files = defaults.zip_mod_files;
        display_vec.push(command_output("config.zip_mod_files".to_string(), defaults.zip_mod_files.to_string()));
    }

    if args.backup_mods {
        config.backup_mods = defaults.backup_mods;
        display_vec.push(command_output("config.backup_mods".to_string(), defaults.backup_mods.to_string()));
    }

    if args.pin_game_version {
        config.pinned_game_version.clone_from(&defaults.pinned_game_version);
        display_vec.push(command_output("config.pinned_game_version".to_string(), defaults.pinned_game_version.to_string()));
    }

    if args.mod_dir {
        config.mod_dir.clone_from(&defaults.mod_dir);
        display_vec.push(command_output("config.mod_dir".to_string(), defaults.mod_dir.to_string()));
    }

    if args.notify_of_unzipped_mods {
        config.notify_of_unzipped_mods = defaults.notify_of_unzipped_mods;
        display_vec.push(command_output("config.notify_of_unzipped_mods".to_string(), config.notify_of_unzipped_mods.to_string()));
    }


    display_table(display_vec, None);
    config.save(None).unwrap();
}

async fn list() {
    let config = get_config().read().await;
    let display_vec: Vec<(CellData, CellData)> = vec![
        command_output("config.mod_dir".to_string(), config.mod_dir.to_string()),
        command_output("config.backup_mods".to_string(), config.backup_mods.to_string()),
        command_output("config.backup_mods_dir".to_string(), config.backup_mods_dir.to_string()),
        command_output("config.zip_mod_files".to_string(), config.zip_mod_files.to_string()),
        command_output("config.show_execution_time".to_string(), config.show_execution_time.to_string()),
        command_output("config.notify_of_unzipped_mods".to_string(), config.notify_of_unzipped_mods.to_string()),
        command_output("config.pinned_game_version".to_string(), config.pinned_game_version.to_string()),
    ];
    
    display_table(display_vec, None);
}