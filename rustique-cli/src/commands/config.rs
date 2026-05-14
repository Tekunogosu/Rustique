use crate::commands::arg_structs::config_args::{DelArgs, CommonArgs, ConfigCommand, ConfigSubCommand};
use rustique_core::utils::get_expanded_path;
use std::path::PathBuf;
use std::process::exit;
use comfy_table::{Attribute, CellAlignment, Color, ContentArrangement, Row, Table};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_FULL_CONDENSED};
use semver::VersionReq;
use tracing::{warn};
use crate::commands::config_table::config_table;
use rustique_core::config::config_manager::{get_config, Config, Package};
use rustique_core::config::config_structs::{CellAttr, CellColor};
use rustique_core::information_utils::{command_output, display_table, notice, prep_cell, CellData};

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

           display_vec.push(command_output("config.mods_dir", path));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }
    
    if let Some(path) = &args.modpacks_dir {
        let dir = get_expanded_path(PathBuf::from(path));
        if dir.exists() {
            config.modpacks.modpack_dir = dir.to_string_lossy().to_string();
            save = true;
            
            display_vec.push(command_output("config.modpacks_dir", path));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }

    if let Some(notif) = &args.notify_of_unzipped_mods {
        config.notify_of_unzipped_mods = *notif;
        save = true;
        display_vec.push(command_output("config.show_mod_dir_warning", notif.to_string()));
    }

    if let Some(version) = &args.pin_game_version {
        if VersionReq::parse(version).is_err() {
            notice(
                "The version string you tried to pin is invalid. \
                 Valid operators are: <, <=, >, >=, =. \
                 Wildcards (*) are supported for major, minor, or patch sections (e.g., '1.22.*'), \
                 but they cannot be used if the version includes pre-release identifiers (e.g., '-rc', '-pre', '-alpha').",
                Some(Color::Yellow),
                vec![Attribute::Bold]
            );

            return
        }

        config.pinned_game_version.clone_from(version);
        save = true;
        display_vec.push(command_output("config.pinned_game_version", version));
    }

    if let Some(allow_unstable ) = &args.allow_unstable {
        config.allow_unstable = *allow_unstable;
        save = true;
       display_vec.push(command_output("config.allow_unstable", allow_unstable.to_string()));
    }

    if let (Some(with_mod), Some(version)) = (&args.with_mod, &args.pin_version) {

        if VersionReq::parse(version).is_err() {
            notice(
                "The version string you tried to pin is invalid. \
                 Valid operators are: <, <=, >, >=, =. \
                 Wildcards (*) are supported for major, minor, or patch sections (e.g., '0.1.*', '0.*.0'), \
                 but they cannot be used if the version includes pre-release identifiers (e.g., '-rc', '-pre', '-alpha').",
                Some(Color::Yellow),
                vec![Attribute::Bold]
            );
            return
        }

        if let Some(pkg) = config.pkg.iter_mut().find(|p| p.mod_id.eq_ignore_ascii_case(with_mod)) {
            pkg.pinned_version = Some(version.clone());
        } else {
            config.pkg.push(Package {
                mod_id: with_mod.clone(),
                pinned_version: Some(version.clone()),
            });
        }

        save = true;
        display_vec.push(command_output(format!("Pinned: {with_mod}"), version));
        notice("Be sure to run the sync command to update Rustique's sync file to use the newly set pinned mod version.", Some(Color::Green), vec![]);
    }

    if let Some(val) = &args.show_execution_time {

        config.show_execution_time = *val;
        save = true;

        display_vec.push(command_output("config.show_execution_time", val.to_string()));
    }
    
    if let Some(backup) = &args.backup_mods {
        config.backup_mods = *backup;
        save = true;

        display_vec.push(command_output("config.backup_mods", backup.to_string()));
    }

    if let Some(backup_dir) = &args.backup_mods_dir {
        let dir = get_expanded_path(PathBuf::from(backup_dir));
        if dir.exists() {
            config.backup_mods_dir = dir.to_string_lossy().to_string();
            save = true;

            display_vec.push(command_output("config.backup_mods_dir", dir.to_string_lossy().to_string()));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }
    
    if let Some(download_dir) = &args.game_download_dir {
        let dir = get_expanded_path(PathBuf::from(download_dir));
        if dir.exists() {
            config.game_download_dir = dir.to_string_lossy().to_string();
            save = true;
            
            display_vec.push(command_output("config.game_download_dir", download_dir));
        } else {
            warn!("{} is not a valid directory", dir.to_string_lossy());
        }
    }
    
    if let Some(mpk_id)  = &args.modpack_enabled {
        if config.modpacks.enabled.contains(mpk_id) {
            display_vec.push(command_output("This modpack ID is already part of config.modpacks.enabled", mpk_id));
        } else {
            config.modpacks.enabled.push(mpk_id.clone());
            save = true;
            display_vec.push(command_output("config.modpacks.enabled", config.modpacks.enabled.join(", ")));
        }
    }
    
    if let Some(mpk_id)  = &args.modpack_disabled {
        if config.modpacks.disabled.contains(mpk_id) {
            display_vec.push(command_output("This modpack ID is already part of config.modpacks.disabled", mpk_id));
        } else {
            config.modpacks.disabled.push(mpk_id.clone());
            save = true;
            display_vec.push(command_output("config.modpacks.disabled", config.modpacks.disabled.join(", ")));
        }
    } 
    
    if let Some(check) = args.check_for_updates {
        config.check_for_updates = check; 
        save = true;
        display_vec.push(command_output("config.check_for_updates", args.check_for_updates.unwrap_or_default().to_string()));
    }

    #[cfg(windows)]
    if let Some(show) = args.update_default_windows_loc {
        config.update_default_windows_loc = show;
        save = true;
        display_vec.push(command_output("config.update_default_windows_loc", show.to_string()));
    }
    
    if !display_vec.is_empty() {
        display_table(display_vec, None);
    }

    if save {
        config.save(None).unwrap();
    }
}

async fn del(args: &DelArgs) {

    let mut config = get_config().write().await;
    let defaults = Config::default();
    let mut display_vec: Vec<(CellData, CellData)> = Vec::new();

    let mut save = false;

    if args.backup_mods_dir {
        config.backup_mods_dir.clone_from(&defaults.backup_mods_dir);
        save = true;
        display_vec.push(command_output("config.backup_mods_dir", defaults.backup_mods_dir));
    }
    
    if args.modpack_dir {
        config.modpacks.modpack_dir.clone_from(&defaults.modpacks.modpack_dir);
        save = true;
        display_vec.push(command_output("config.modpacks.modpack_dir", defaults.modpacks.modpack_dir));
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

    if args.allow_unstable {
        config.allow_unstable.clone_from(&defaults.allow_unstable);
        save = true;
        display_vec.push(command_output("config.allow_unstable", config.allow_unstable.to_string()));
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
    
    if let Some(mpk_id) = &args.modpack_enabled {
        if config.modpacks.enabled.contains(mpk_id) {
            config.modpacks.enabled.retain(|m| m != mpk_id);
            save = true;
            display_vec.push(command_output("config.modpacks.enabled", config.modpacks.enabled.join(", ")));
        } else {
            display_vec.push(command_output("This modpack id is not in the enabled list", mpk_id)); 
        }
    }
    
    if let Some(mpk_id) = &args.modpack_disabled {
        if config.modpacks.disabled.contains(mpk_id) {
            config.modpacks.disabled.retain(|m| m != mpk_id);
            save = true;
            display_vec.push(command_output("config.modpacks.disabled", config.modpacks.disabled.join(", ")));
        } else {
            display_vec.push(command_output("This modpack id is not in the disabled list", mpk_id)); 
        }
    }
    
    if args.check_for_updates {
        config.check_for_updates = true;
        save = true;
        display_vec.push(command_output("config.check_for_updates", config.check_for_updates.to_string()));
    }
    
    if !display_vec.is_empty() { 
        display_table(display_vec, None);
    }
    
    if save {
        config.save(None).unwrap();
    }
}

async fn list() {
    let config = get_config().read().await;
    let display_vec: Vec<(CellData, CellData)> = vec![
        command_output("config.mod_dir",                 &config.mod_dir),
        command_output("config.backup_mods_dir",         &config.backup_mods_dir),
        command_output("config.game_download_dir",       &config.game_download_dir),
        command_output("config.backup_mods",             config.backup_mods.to_string()),
        command_output("config.show_execution_time",     config.show_execution_time.to_string()),
        command_output("config.notify_of_unzipped_mods", config.notify_of_unzipped_mods.to_string()),
        command_output("config.pinned_game_version",     &config.pinned_game_version),
        command_output("config.allow_unstable",          config.allow_unstable.to_string()),
        command_output("",""),
        command_output("config.check_for_updates",       config.check_for_updates.to_string()),

        #[cfg(windows)] 
        command_output("config.update_windows_default_loc", config.update_default_windows_loc.to_string()),

        command_output("",""),
        command_output("config.modpacks.modpack_dir",    &config.modpacks.modpack_dir),
        command_output("config.modpacks.enabled",        config.modpacks.enabled.join(",")),
        command_output("config.modpacks.disabled",       config.modpacks.disabled.join(",")),
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
            let mod_name = prep_cell(pkg.mod_id.clone(), Some(CellColor::Yellow), None, None, None);
            let pinned_version = prep_cell(pkg.pinned_version.clone().unwrap_or(String::new()), Some(CellColor::Magenta), None, None, Some(CellAlignment::Right));
            rows.push(Row::from(vec![mod_name, pinned_version]));
        }

        table.add_rows(rows);
        println!("{table}");
    }
}