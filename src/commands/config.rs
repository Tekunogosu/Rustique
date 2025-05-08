use crate::commands::arg_structs::config_args::{BoolArgs, CommonArgs, ConfigCommand, ConfigSubCommand};
use crate::config_manager::{get_config};
use crate::utils::{command_output, display_table, get_expanded_path, CellData};
use std::path::PathBuf;
use tracing::{warn};

pub fn parse_config_args(config_cmd: &ConfigCommand) {
    match &config_cmd.subcommand {
        ConfigSubCommand::Set(args) => {
            set(&args.common)
        },
        ConfigSubCommand::List => {
            println!("listing all configurations");
        },
        ConfigSubCommand::Show(_args) => {
            // show(&args.common);
        },
        ConfigSubCommand::Del(args) => {
            println!("{:?}", args);
        },
    }
}


fn set(args: &CommonArgs) {

    let mut config = get_config().write().unwrap();

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

    if let Some(show) = &args.notify_of_unzipped_mods {
        config.notify_of_unzipped_mods = *show;
        config.save(None).unwrap();

       display_vec.push(command_output("config.show_mod_dir_warning".to_string(), show.to_string()));
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

// fn show(args: &CommonArgs) {
//
//         if let Some(mod_dir) = {}
//         CommonArgs { pin_game_version: String, ..} => {}
//         CommonArgs { zip_mod_dirs: bool, .. } => {}
//         CommonArgs { backup_mods: bool, .. } => {}
//         CommonArgs { backup_mods_dir: String, ..} => {}
// }

fn _list() {
    println!("listing all configurations...");
}

fn _del(args: &BoolArgs) {
    if args.backup_mods_dir {

    }

    if args.zip_mod_dirs {

    }

    if args.backup_mods {

    }

    if args.pin_game_version {

    }

    if args.mods_dir {

    }

}
