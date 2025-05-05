#![allow(unused_imports, dead_code)]

mod utils;
mod api_structs;
mod api;
mod cli_commands;
mod rustique_errors;

mod aliases;
mod version_management;
mod commands;
mod logging;
mod config_manager;

use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::process::exit;
use std::time::Instant;
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use colored::Colorize;
use tracing::field::debug;
use tracing::{debug, Level, error, info, trace, warn};
use tracing_subscriber::{fmt, EnvFilter, prelude::*};
use crate::aliases::ModID;
// use commands::bulk_downloader::bulk_download;
use crate::cli_commands::{Cli, Commands};
use crate::commands::list::list_installed;
use commands::install::{install_missing_dependencies, install_mods, InstallOrUpdate};
use crate::utils::{elapsed_footer, get_expanded_path, RustiqueOptions};
use commands::sync::sync;
use commands::update::update_mods;
use crate::commands::arg_structs::modpack_args::ModpackCommands;
use crate::commands::config::{parse_config_args};
// use crate::commands::sync::handle_sync_call;
use crate::config_manager::{get_config, init_config};
use crate::logging::{init_logging, VerboseLevel};



fn main() {
    // Initialize the Tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Run our async main function in the runtime
    rt.block_on(async_main());
}

async fn async_main() {
    let cli = Cli::parse();
    let verbosity = if cli.debug {
        VerboseLevel::Debug
    } else if cli.verbose {
        VerboseLevel::Verbose
    } else {
        VerboseLevel::Default
    };
    init_logging(verbosity);
    // setup the config global
    // ideally this *could* be setup by the user on where they want the config to be loaded from,
    // but for now it will always be in .config/rustique
    // this will need to be modified to work with windows using %appdata%
    match init_config(None) {
        Ok(_) => {},
        Err(e) => {
            debug!("{}", e.to_string().red().bold());
        }
    }
    if cli.verbose {
        debug!("Verbose logging enabled");
    }
    let mut mod_opts: RustiqueOptions = RustiqueOptions::default();
    let mut mod_dir = mod_opts.get_mod_path();
    // the mods_dir from the cli takes priority from all other means, including the config file
    if cli.mods_dir.is_some() {
        mod_dir = get_expanded_path(PathBuf::from(cli.mods_dir.unwrap()));
    }
    info!("Operating on mods dir: {:?}", mod_opts.mod_dir);
    // TODO: check for windows equiv
    match &cli.command {
        Commands::Sync => {
            // Sync will add a rustique-sync.json to a valid mod_dir
            handle_sync_call(&mod_dir).await;
        }
        Commands::List(args) => {
            match list_installed(&mod_dir, args.updates).await {
                Ok(_) => {}
                Err(e) => {
                    error!("{}", e.to_string().red().bold());
                    exit(1);
                }
            }
        }
        Commands::Update(args) => {
            match update_mods(&mod_dir, args.mod_ids.clone(), args.keep_old_files).await {
                Ok(_) => {
                    handle_sync_call(&mod_dir).await;
                }
                Err(e) => {
                    warn!("{}", e.to_string().red().bold());
                    exit(1);
                }
            }
        }
        Commands::Changelog(name) => {
            println!("list {:?}", name.name);
        }
        Commands::Install(args) => {
            let start_time = Instant::now();
            let config = get_config().read().unwrap();
            if args.mod_ids.len() > 0 {
                let mod_ids: HashSet<ModID> = args.mod_ids.iter().cloned().collect();
                match install_mods(&mod_dir, InstallOrUpdate::Install(mod_ids)).await {
                    Ok(_) => {
                        if args.mod_ids.len() > 1 {
                            info!("Mods successfully installed!");
                        } else {
                            info!("Mods successfully installed!");
                        }
                        handle_sync_call(mod_opts.mod_dir.as_ref().unwrap()).await;
                    }
                    Err(e) => {
                        error!("Error attempting to install {:?} : {}", args.mod_ids, e.to_string());
                        exit(1);
                    }
                }
            }
            if args.missing_dependencies {
                match install_missing_dependencies(&mod_dir, None).await {
                    Ok(_) => {
                        info!("{}", "All dependencies resolved..".bold().green());
                    }
                    Err(e) => {
                        error!("{}", e.to_string());
                        exit(1);
                    }
                }
            }
            if config.show_execution_time {
                elapsed_footer(start_time, "Install");
            }
        }
        Commands::Config(config_cmd) => {
            parse_config_args(config_cmd);
        }
        Commands::Info(args) => {
            info!("displaying stuff about the mod {:?}", args.mod_id);
        }
        Commands::Search(_args) => {
            info!("Searching stuff");
        }
        Commands::ModPack{command} => {
            match command {
                ModpackCommands::Create(args) => {
                    if args.mod_dir.is_some() {
                        println!("Creating mod pack from {}", mod_dir.as_path().display());
                    }
                    println!("creating modpack with name: {}", &args.name);
                }
            }
        }
        // #[cfg(feature = "dev")]
        // Commands::BulkDownloader(args) => {
        //     match bulk_download(&mod_dir, args.num_to_download).await {
        //         Ok(_) => {
        //             info!("All mods downloaded.. hopefully..");
        //         }
        //         Err(e) => {
        //             error!("{}", e.to_string());
        //         }
        //     }
        // }
        #[cfg(feature = "dev")]
        Commands::TestCommand(args) => {
            {
                let mut config = get_config().write().unwrap();
                config.pinned_game_version = args.version_to_pin.to_string();
                config.save(None).unwrap();
            }
            {
                let config = get_config().read().unwrap();
                info!("{:?}", config);
            }
        }
        #[cfg(feature = "dev")]
        Commands::LoadMods(args) => {
            let file_path = get_expanded_path(PathBuf::from(args.filename.clone()));

            // Use tokio's async file reading
            match tokio::fs::read_to_string(file_path).await {
                Ok(contents) => {
                    let list: Vec<String> = contents.split('\n').map(|s| s.to_string()).collect();
                    info!("{:?}", list);
                    info!("COUNT: {}", list.len());
                    let set: HashSet<String> = HashSet::from_iter(list);
                    if let Err(e) = install_mods(&mod_dir, InstallOrUpdate::Install(set)).await {
                        error!("Failed to install mods: {}", e);
                    }
                },
                Err(e) => {
                    error!("Failed to read file: {}", e);
                }
            }
        }
        _ => {}
    }
}

// Update this function to be async
async fn handle_sync_call(mod_dir: &PathBuf) {
    match sync(mod_dir).await {
        Ok(_) => {},
        Err(e) => {
            error!("{}", e.to_string().red().bold());
            exit(1);
        }
    }
}

//
// // TODO: Add feature to notify user when the modinfo.json file is malformed
// fn main() {
//
//     let cli = Cli::parse();
//
//     let verbosity = if cli.debug {
//         VerboseLevel::Debug
//     } else if cli.verbose {
//         VerboseLevel::Verbose
//     } else {
//         VerboseLevel::Default
//     };
//
//     init_logging(verbosity);
//
//     // setup the config global
//     // ideally this *could* be setup by the user on where they want the config to be loaded from,
//     // but for now it will always be in .config/rustique
//     // this will need to be modified to work with windows using %appdata%
//     match init_config(None) {
//         Ok(_) => {},
//         Err(e) => {
//             debug!("{}", e.to_string().red().bold());
//         }
//     }
//
//     if cli.verbose {
//         debug!("Verbose logging enabled");
//     }
//
//     let mut mod_opts: RustiqueOptions = RustiqueOptions::default();
//
//     let mut mod_dir = mod_opts.get_mod_path();
//
//     // the mods_dir from the cli takes priority from all other means, including the config file
//     if cli.mods_dir.is_some() {
//         mod_dir = get_expanded_path(PathBuf::from(cli.mods_dir.unwrap()));
//     }
//
//     info!("Operating on mods dir: {:?}", mod_opts.mod_dir);
//
//     // TODO: check for windows equiv
//     match &cli.command {
//         Commands::Sync => {
//             // Sync will add a rustique-sync.json to a valid mod_dir
//             handle_sync_call(&mod_dir);
//         }
//         Commands::List(args) => {
//             match list_installed(&mod_dir, args.updates) {
//                 Ok(_) => {}
//                 Err(e) => {
//                     error!("{}", e.to_string().red().bold());
//                     exit(1);
//                 }
//             }
//         }
//         Commands::Update(args) => {
//             match update_mods(&mod_dir, args.mod_ids.clone(), args.keep_old_files) {
//                 Ok(_) => {
//                     handle_sync_call(&mod_dir);
//                 }
//                 Err(e) => {
//                     warn!("{}", e.to_string().red().bold());
//                     exit(1);
//                 }
//             }
//         }
//         Commands::Changelog(name) => {
//             println!("list {:?}", name.name);
//         }
//         Commands::Install(args) => {
//             let start_time = Instant::now();
//             let config = get_config().read().unwrap();
//
//             if args.mod_ids.len() > 0 {
//                 let mod_ids: HashSet<ModID> = args.mod_ids.iter().cloned().collect();
//
//                 match install_mods(&mod_dir, InstallOrUpdate::Install(mod_ids)) {
//                     Ok(_) => {
//                         if args.mod_ids.len() > 1 {
//                             // eprintln!("{}", "Mods successfully installed!".bold().green());
//                             info!("Mods successfully installed!");
//                         } else {
//                             // eprintln!("{}", "Mod successfully installed!".bold().green());
//                             info!("Mods successfully installed!");
//                         }
//
//                         handle_sync_call(mod_opts.mod_dir.as_ref().unwrap());
//                     }
//                     Err(e) => {
//                         error!("Error attempting to install {:?} : {}", args.mod_ids, e.to_string());
//                         exit(1);
//                     }
//                 }
//
//             }
//
//             if args.missing_dependencies {
//
//                 match install_missing_dependencies(&mod_dir, None) {
//                     Ok(_) => {
//                         info!("{}", "All dependencies resolved..".bold().green());
//                     }
//                     Err(e) => {
//                         error!("{}", e.to_string());
//                         exit(1);
//                     }
//                 }
//             }
//
//             if config.show_execution_time {
//                 elapsed_footer(start_time, "Install");
//             }
//         }
//
//         Commands::Config(config_cmd) => {
//             parse_config_args(config_cmd);
//         }
//
//         Commands::Info(args) => {
//             info!("displaying stuff about the mod {:?}", args.mod_id);
//         }
//         Commands::Search(_args )=> {
//             info!("Searching stuff");
//         }
//         Commands::ModPack{command} => {
//             match command {
//                 ModpackCommands::Create(args) => {
//                     if args.mod_dir.is_some() {
//                         println!("Creating mod pack from {}", mod_dir.as_path().display());
//                     }
//
//                     println!("creating modpack with name: {}", &args.name);
//                 }
//             }
//         }
//         #[cfg(feature = "dev")]
//         Commands::BulkDownloader(args) => {
//             match bulk_download(&mod_dir, args.num_to_download) {
//                 Ok(_) => {
//                     info!("All mods downloaded.. hopefully..");
//                 }
//                 Err(e) => {
//                     error!("{}", e.to_string());
//                 }
//             }
//         }
//
//         #[cfg(feature = "dev")]
//         Commands::TestCommand(args) => {
//             {
//                 let mut config = get_config().write().unwrap();
//                 config.pinned_game_version = args.version_to_pin.to_string();
//                 config.save(None).unwrap();
//             }
//
//             {
//                let config = get_config().read().unwrap();
//                 info!("{:?}", config);
//             }
//         }
//         #[cfg(feature = "dev")]
//         Commands::LoadMods(args) => {
//             let file_path = get_expanded_path(PathBuf::from(args.filename.clone()));
//             let mut contents = String::new();
//             let mut mod_file = File::open(file_path).unwrap();
//             mod_file.read_to_string(&mut contents).unwrap();
//
//             let list : Vec<String> = contents.split('\n').map(|s| s.to_string()).collect();
//
//             info!("{:?}", list);
//             info!("COUNT: {}", list.len());
//
//             let set : HashSet<String> = HashSet::from_iter(list);
//
//             install_mods(&mod_dir, InstallOrUpdate::Install(set)).unwrap();
//         }
//
//
//     }
// }
