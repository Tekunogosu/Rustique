#![allow(unused_imports, dead_code)]

mod sync;
mod list;
mod update;
mod changelog;
mod install;
mod utils;
mod api_structs;
mod api;
mod cli_commands;
mod modpack_commands;
mod rustique_errors;

mod aliases;
mod bulk_downloader;
mod version_management;
mod config_structs;
mod config;

use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::exit;
use clap::{Args, Parser, Subcommand, ColorChoice, CommandFactory, FromArgMatches, crate_authors};
use colored::Colorize;
use crate::aliases::ModID;
use crate::bulk_downloader::bulk_download;
use crate::cli_commands::{Cli, Commands};
use crate::config_structs::{get_config, init_config};
use crate::install::{install_missing_dependencies, install_mod, install_mods, InstallOrUpdate};
use crate::utils::{dlog, get_expanded_path, RustiqueOptions};
use crate::list::list_installed;
use crate::modpack_commands::ModpackCommands;
use crate::sync::sync;
use crate::update::{update_mods};

// TODO: Add feature to notify user when the modinfo.json file is malformed
fn main() {

    let cli = Cli::parse();

    // setup the config global
    // ideally this *could* be setup by the user on where they want the config to be loaded from,
    // but for now it will always be in .config/rustique
    // this will need to be modified to work with windows using %appdata%
    match init_config(None) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("{}", e.to_string());
        }
    }

    let mod_opts = if cli.mods_dir.is_none() {
        RustiqueOptions::default()
    } else {
        RustiqueOptions {
            mod_dir: Some(get_expanded_path(PathBuf::from(cli.mods_dir.unwrap()))),
            mod_id: None
        }
    };

    // TODO: check for windows equiv
    match &cli.command {
        Commands::Sync => {
            // Sync will add a rustique-sync.json to a valid mod_dir
            handle_sync_call(mod_opts.mod_dir.as_ref().unwrap());
        }
        Commands::List(args) => {
            match list_installed(mod_opts.mod_dir.as_ref().unwrap(), args.updates) {
                Ok(_) => {}
                Err(e) => {
                    print!("{}", e.to_string());
                    exit(1);
                }
            }
        }
        Commands::Update(args) => {
            match update_mods(mod_opts.mod_dir.as_ref().unwrap(), args.mod_ids.clone(), args.keep_old_files) {
                Ok(_) => {
                    handle_sync_call(mod_opts.mod_dir.as_ref().unwrap());
                }
                Err(e) => {
                    print!("{}", e.to_string());
                    exit(1);
                }
            }
        }
        Commands::Changelog(name) => {
            println!("list {:?}", name.name);
        }
        Commands::Install(args) => {
            if args.mod_ids.len() > 0 {
                let mod_ids: HashSet<ModID> = args.mod_ids.iter().cloned().collect();
                match install_mods(mod_opts.mod_dir.as_ref().unwrap(), InstallOrUpdate::Install(mod_ids)) {
                    Ok(_) => {
                        if args.mod_ids.len() > 1 {
                            eprintln!("{}", "Mods successfully installed!".bold().green());
                        } else {
                            eprintln!("{}", "Mod successfully installed!".bold().green());
                        }

                        handle_sync_call(mod_opts.mod_dir.as_ref().unwrap());
                    }
                    Err(e) => {
                        println!("Error attempting to install {:?} : {}", args.mod_ids, e.to_string());
                        exit(1);
                    }
                }
            }

            if args.missing_dependencies {

                match install_missing_dependencies(mod_opts.mod_dir.as_ref().unwrap(), None) {
                    Ok(_) => {
                        eprintln!("{}", "All dependencies resolved..".bold().green());
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                        exit(1);
                    }
                }
            }
        }
        Commands::Info(args) => {
            println!("displaying stuff about the mod {:?}", args.mod_id);
        }
        Commands::Search(_args )=> {
            print!("Searching stuff");
        }
        Commands::ModPack{command} => {
            match command {
                ModpackCommands::Create(args) => {
                    if args.mod_dir.is_some() {
                        println!("Creating mod pack from {}", &args.mod_dir.as_ref().unwrap().to_string());
                    }

                    println!("creating modpack with name: {}", &args.name);
                }
            }
        }
        #[cfg(feature = "dev")]
        Commands::BulkDownloader(args) => {
            match bulk_download(mod_opts.mod_dir.as_ref().unwrap(), args.num_to_download) {
                Ok(_) => {
                    println!("All mods downloaded.. hopefully..");
                }
                Err(e) => {
                    eprintln!("{}", e.to_string());
                }
            }
        }

        #[cfg(feature = "dev")]
        Commands::TestCommand(args) => {
            {
                let mut config = get_config().write().unwrap();
                config.pinned_game_version = args.version_to_pin.to_string();
                config.save(None).unwrap();
            }

            {
               let config = get_config().read().unwrap();
                eprintln!("{:?}", config);
            }
        }
        #[cfg(feature = "dev")]
        Commands::LoadMods(args) => {
            let file_path = get_expanded_path(PathBuf::from(args.filename.clone()));
            let mut contents = String::new();
            let mut mod_file = File::open(file_path).unwrap();
            mod_file.read_to_string(&mut contents).unwrap();

            let list : Vec<String> = contents.split('\n').map(|s| s.to_string()).collect();

            let set : HashSet<String> = HashSet::from_iter(list);

            install_mods(mod_opts.mod_dir.as_ref().unwrap(), InstallOrUpdate::Install(set)).unwrap();
        }
    }
}

fn handle_sync_call(mod_dir: &PathBuf) {
    match sync(mod_dir) {
        Ok(_) => {}
        Err(e) => {
           println!("{}", e.to_string());
            exit(1);
        }
    }
}