#![feature(path_add_extension)]
#![warn(clippy::perf, clippy::pedantic)]
#![warn(clippy::manual_string_new)]
#![allow(clippy::redundant_closure_for_method_calls, clippy::struct_field_names, clippy::doc_markdown, clippy::unnecessary_wraps)]

mod utils;
mod api;
mod cli_commands;
mod rustique_errors;

mod aliases;
mod version_management;
mod commands;
mod logging;
mod install_manager;
mod traits;
mod information_utils;
mod modpack;
mod config;
mod consts;
mod updater;

use std::env::args;
use crate::cli_commands::{Cli, Commands, ShellType};
use config::config::parse_config_args;
use crate::commands::install::{install_cmd, install_missing_deps};
use crate::commands::list::cmd_list;
use crate::commands::sync::{daily_file_syncs, game_version_sync};
use crate::logging::{init_logging, VerboseLevel};
use crate::utils::{get_expanded_path, sorted_game_versions, RustiqueOptions};
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use owo_colors::OwoColorize;
use commands::sync::sync;
use commands::update::update_mods;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::Instant;
use comfy_table::{Attribute, Color};
use tracing::{debug, error, info, warn};
use crate::commands::download::download;
use crate::commands::info::info;
use crate::commands::search::search;
use crate::config::config_manager::{get_config, init_config};
use crate::information_utils::{elapsed_footer, notice};
use crate::modpack::modpack_commands::parse_modpack_commands;
use crate::traits::ref_ext::PathRef;
use crate::traits::string_ext::StrLowerExt;
use crate::updater::update_manager;
use crate::updater::update_manager::check_for_update;

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
    init_logging(&verbosity);
    // setup the config global
    // ideally this *could* be setup by the user on where they want the config to be loaded from,
    // but for now it will always be in .config/rustique
    // this will need to be modified to work with windows using %appdata%
    match init_config(None) {
        Ok(()) => {},
        Err(e) => {
            debug!("{}", e.to_string().red().bold());
        }
    }
    if cli.verbose {
        debug!("Verbose logging enabled");
    }
    let mod_opts: RustiqueOptions = RustiqueOptions::default();
    let mut mod_dir = mod_opts.get_mod_path().await;
    // the mods_dir from the cli takes priority from all other means, including the config file
    if cli.mods_dir.is_some() {
        mod_dir = get_expanded_path(PathBuf::from(cli.mods_dir.clone().unwrap_or(String::new())));
        if !mod_dir.exists() {
            notice("The directory you specified is not valid. Check your input for typos and try again.", Some(Color::Yellow), vec![Attribute::Bold]); 
            exit(1);
        }
    }

    info!("Operating on mods dir: {:?}", mod_dir);


    // don't display the update message we are calling anything with self as it already deall with updates
    if !matches!(&cli.command, Commands::RustiqueSelf(_)) {
        let _ = check_for_update(false, true).await;
    }

    if cli.with_mpk.is_some() {
        let config = get_config().read().await;
        mod_dir = Path::new(&config.modpacks.modpack_dir).join("installed").join(cli.with_mpk.clone().unwrap_or(String::new()));
        if !mod_dir.exists() {
            notice("The modpack you specified isn't installed. Double check your spelling and try again.", Some(Color::Yellow), vec![Attribute::Bold]);
            exit(1);
        }
    }


    match &cli.command {
        Commands::Sync(args) => {
            // Sync will add a rustique-sync.json to a valid mod_dir
            if args.sync_search_db {
                match daily_file_syncs(args.sync_search_db).await {
                    Ok(_) => {},
                    Err(e) => {
                        error!("{}", e.to_string().red().bold());
                    }
                }
            } else if args.sync_game_versions {
                match game_version_sync(args.sync_game_versions).await {
                    Ok(_) => {},
                    Err(e) => {
                        error!("{}", e.to_string().red().bold());
                    }
                }
            } else {
                handle_sync_call(&mod_dir, false).await;
            }
        }
        Commands::List(args) => {
            if args.game_versions.is_some() {
                let sorted_versions = sorted_game_versions().await;
                let filter_by = &args.game_versions.clone().unwrap_or("1.20".into());
                
                let versions: Vec<String> = sorted_versions.into_iter().filter(|v| v.lower_contains(filter_by)).collect();
                
               notice(format!("[{}]",versions.join("], [").as_str()), Some(Color::Yellow), vec![]); 
                
            } else {
                match cmd_list(&mod_dir, args.updates, false, false).await {
                    Ok(()) => {

                    },
                    Err(e) => {
                        error!("{}", e.to_string().red().bold());
                    }
                }
            }
        }
        Commands::Update(args) => {
            match update_mods(&mod_dir, args.mod_ids.clone(), args.keep_old_files).await {
                Ok(()) => {
                    handle_sync_call(&mod_dir, false).await;
                }
                Err(e) => {
                    warn!("{}\n\r", e.to_string().red().bold());
                    exit(1);
                }
            }
        }
        Commands::Download(args) => {
            match download(args).await {
                Ok(()) => {},
                Err(e) => {
                    eprint!("{}", e.to_string().red().bold());
                }
            }
        }
        Commands::Install(args) => {
            let start_time = Instant::now();
            let config = get_config().read().await;

            

            if !args.mod_ids.is_empty() {
                match install_cmd(&mod_dir, args.mod_ids.clone(), args.missing_dependencies).await {
                    Ok(()) => {
                        handle_sync_call(&mod_dir, false).await;
                    }
                    Err(e) => {
                        error!("{}", e);
                    }
                }
            }
            
            if args.missing_dependencies {
                match install_missing_deps(&mod_dir, args.mod_ids.clone()).await {
                    Ok(()) => {
                        handle_sync_call(&mod_dir, false).await;
                    },
                    Err(e) => {
                        error!("{}", e);
                    }
                }
            }
            
            if config.show_execution_time {
                elapsed_footer(start_time, "Install");
            }
        }
        Commands::Config(config_cmd) => {
            parse_config_args(config_cmd).await;
        }
        Commands::Misc{ gen_auto_complete: Some(shell) } => {
                generate_completion(shell.clone());
        }
        Commands::Info(args) => {
            match info(args).await {
                Ok(()) => {}
                Err(e) => {
                    error!("{}", e.to_string().red().bold());
                }
            }
        }
        Commands::Search(args) => match search(args).await {
            Ok(()) => {}
            Err(e) => {
                error!("{}", e.to_string().red().bold());
            }
        },
        Commands::Modpack(cmds) => {
           parse_modpack_commands(cmds, &mod_dir).await;
        }
        Commands::Misc{ .. }=> {},
        Commands::RustiqueSelf(args) =>{
            if args.check_updates {
                match update_manager::check_for_update(false, false).await {
                    Ok(_) => {
                       // Since this is a direct check for update, we don't do anything with the returned bool. 
                    },
                    Err(e) => {
                        error!("{}", e.to_string().red().bold());
                    }
                }
            }
            
            if args.update {
                match update_manager::self_update_binary(args.force).await {
                    Ok(()) => {}
                    Err(e) => {
                       error!("{}", e.to_string().red().bold()); 
                    }
                }
            }
        }
        
    }
}

// Update this function to be async
async fn handle_sync_call(mod_dir: impl PathRef, quiet: bool) {
    match sync(mod_dir.as_ref(), quiet, vec![]).await {
        Ok(()) => {},
        Err(e) => {
            error!("{}", e.to_string().red().bold());
            exit(1);
        }
    }
}

fn generate_completion(shell: ShellType) {
    let mut cmd = Cli::command();
    let shell: Shell = shell.into();

    // Generate the completion script to stdout
    generate(shell, &mut cmd, "Rustique", &mut io::stdout());

    println!("\n# Completion script generated. To use it:");
    match shell {
        Shell::Bash => {
            println!("# Save the above output to ~/.local/share/bash-completion/completions/Rustique");
            println!("# Or run: Rustique misc --gen-auto-complete bash > ~/.local/share/bash-completion/completions/Rustique");
        }
        Shell::Zsh => {
            println!("# Save the above output to ~/.zsh/completion/_Rustique");
            println!("# Or run: Rustique misc --gen-auto-complete zsh > ~/.zsh/completion/_Rustique");
            println!("# Then add to your .zshrc: fpath=(~/.zsh/completion $fpath)");
        }
        Shell::Fish => {
            println!("# Save the above output to ~/.config/fish/completions/Rustique.fish");
            println!("# Or run: Rustique misc --gen-auto-complete fish > ~/.config/fish/completions/Rustique.fish");
        }
        Shell::PowerShell => {
            println!("# Save the above output to a file and source it in your PowerShell profile");
            println!("# Or run: Rustique misc --gen-auto-complete powershell > Rustique.ps1");
        }
        _ => {}
    }
}