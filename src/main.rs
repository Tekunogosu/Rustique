mod utils;
mod api;
mod cli_commands;
mod rustique_errors;

mod aliases;
mod version_management;
mod commands;
mod logging;
mod config_manager;
mod install_manager;

use crate::cli_commands::{Cli, Commands, ShellType};
use crate::commands::arg_structs::modpack_args::ModpackCommands;
use crate::commands::config::parse_config_args;
use crate::commands::install::{install_cmd, install_missing_deps};
use crate::commands::list::list_installed;
use crate::commands::sync::mod_id_sync;
use crate::config_manager::{get_config, init_config};
use crate::logging::{init_logging, VerboseLevel};
use crate::utils::{elapsed_footer, get_expanded_path, RustiqueOptions};
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use colored::Colorize;
use commands::sync::sync;
use commands::update::update_mods;
use std::io;
use std::path::PathBuf;
use std::process::exit;
use std::time::Instant;
use tracing::{debug, error, info, warn};


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
    let mod_opts: RustiqueOptions = RustiqueOptions::default();
    let mut mod_dir = mod_opts.get_mod_path();
    // the mods_dir from the cli takes priority from all other means, including the config file
    if cli.mods_dir.is_some() {
        mod_dir = get_expanded_path(PathBuf::from(cli.mods_dir.clone().unwrap()));
    }

    info!("Operating on mods dir: {:?}", mod_dir);

    // TODO: check for windows equiv
    match &cli.command {
        Commands::Sync(args) => {
            // Sync will add a rustique-sync.json to a valid mod_dir
            if args.ids {
                match mod_id_sync(args.force).await {
                    Ok(_) => {},
                    Err(e) => {
                        error!("{}", e.to_string().red().bold());
                    }
                }
            } else {
                handle_sync_call(&mod_dir).await;
            }

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


            if args.missing_dependencies {
                match install_missing_deps(&mod_dir, args.mod_ids.clone()).await {
                    Ok(_) => {
                        handle_sync_call(&mod_dir).await
                    },
                    Err(e) => {
                        error!("{}", e)
                    }
                }
            }


            if args.mod_ids.len() > 0 {
                match install_cmd(&mod_dir, args.mod_ids.clone(), args.missing_dependencies).await {
                    Ok(_) => {
                        handle_sync_call(&mod_dir).await;
                    }
                    Err(e) => {
                        error!("{}", e)
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
        Commands::Misc{ gen_auto_complete } => {
            if let Some(shell) = gen_auto_complete {
                generate_completion(shell.clone());
            }
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
        // #[cfg(feature = "dev")]
        // Commands::LoadMods(args) => {
        //     let file_path = get_expanded_path(PathBuf::from(args.filename.clone()));
        //
        //     // Use tokio's async file reading
        //     match tokio::fs::read_to_string(file_path).await {
        //         Ok(contents) => {
        //             let list: Vec<String> = contents.split('\n').map(|s| s.to_string()).collect();
        //             info!("{:?}", list);
        //             info!("COUNT: {}", list.len());
        //             let set: HashSet<String> = HashSet::from_iter(list);
        //             if let Err(e) = install_mods(&mod_dir, InstallOrUpdate::Install(set)).await {
        //                 error!("Failed to install mods: {}", e);
        //             }
        //         },
        //         Err(e) => {
        //             error!("Failed to read file: {}", e);
        //         }
        //     }
        // }
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