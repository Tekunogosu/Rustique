#![feature(path_add_extension)]
#![warn(clippy::perf, clippy::pedantic)]
#![warn(clippy::manual_string_new)]
#![allow(
    clippy::redundant_closure_for_method_calls,
    clippy::struct_field_names,
    clippy::doc_markdown,
    clippy::unnecessary_wraps
)]

mod api;
mod cli_commands;
mod rustique_errors;
mod utils;

mod aliases;
mod commands;
mod config;
mod consts;
mod information_utils;
mod install_manager;
mod logging;
mod modpack;
mod rustique_options;
mod traits;
mod updater;
mod version_management;

use crate::cli_commands::{Cli, Commands, ShellType};
use crate::commands::delete::{delete_all, delete_cmd};
use crate::commands::download::download;
use crate::commands::info::info;
use crate::commands::install::{install_cmd, install_missing_deps};
use crate::commands::list::cmd_list;
use crate::commands::search::search;
use crate::commands::sync::{daily_file_syncs, game_version_sync};
use crate::config::config_manager::{get_config, init_config};
use crate::information_utils::{elapsed_footer, notice};
use crate::logging::{VerboseLevel, init_logging};
use crate::modpack::modpack_commands::parse_modpack_commands;
use crate::rustique_errors::{ErrorMsgFn, handle_err_result};
use crate::rustique_options::RustiqueOptions;
use crate::traits::ref_ext::PathRef;
use crate::traits::string_ext::StrLowerExt;
use crate::updater::update_manager;
use crate::updater::update_manager::check_for_update;
use crate::utils::{get_expanded_path, sorted_game_versions};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use comfy_table::{Attribute, Color};
use commands::sync::sync;
use commands::update::update_mods;
use config::config::parse_config_args;
use owo_colors::OwoColorize;
use std::env::home_dir;
use std::fs::File;
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
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
    init_logging(&verbosity);
    // setup the config global
    // ideally this *could* be setup by the user on where they want the config to be loaded from,
    // but for now it will always be in .config/rustique
    handle_err_result(init_config(None), "init_config: ", false, ErrorMsgFn::Debug);

    if cli.verbose {
        info!("Verbose logging enabled");
    }

    if cli.debug {
        debug!("Debug logging enabled");
    }

    // Check if the windows path needs to be updated before we do anything else
    #[cfg(windows)]
    {
        // Prevent the message from popping up if you are calling config.
        // This lets you disable the message WITHOUT being annoyed again by the update call
        if !matches!(cli.command, Commands::Config { .. }) {
            let update_windows_default_loc = {
                let config = get_config().read().await;
                config.update_default_windows_loc
            };

            if update_windows_default_loc {
                match RustiqueOptions::check_old_default_windows().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error attempting to update default mod path {}", e);
                    }
                }
            }
        }
    }

    let mod_opts: RustiqueOptions = RustiqueOptions::default();
    let mut mod_dir = mod_opts.get_mod_path().await;
    // the mods_dir from the cli takes priority from all other means, including the config file
    if cli.mods_dir.is_some() {
        mod_dir = get_expanded_path(PathBuf::from(cli.mods_dir.clone().unwrap_or(String::new())));
        if !mod_dir.exists() {
            notice(
                "The directory you specified is not valid. Check your input for typos and try again.",
                Some(Color::Yellow),
                vec![Attribute::Bold],
            );
            exit(1);
        }
    }

    // don't display the update message we are calling anything with self as it already dealt with updates
    if !matches!(&cli.command, Commands::RustiqueSelf(_)) {
        let config = get_config().read().await;
        let _ = check_for_update(config.check_for_updates, true).await;
    }

    if cli.with_mpk.is_some() {
        let config = get_config().read().await;
        mod_dir = Path::new(&config.modpacks.modpack_dir)
            .join("installed")
            .join(cli.with_mpk.clone().unwrap_or(String::new()));
        if !mod_dir.exists() {
            notice(
                "The modpack you specified isn't installed. Double check your spelling and try again.",
                Some(Color::Yellow),
                vec![Attribute::Bold],
            );
            exit(1);
        }
    }

    info!("Operating on mods dir: {:?}", mod_dir);
    match &cli.command {
        Commands::Sync(args) => {
            // Sync will add a rustique-sync.json to a valid mod_dir
            if args.sync_search_db {
                handle_err_result(
                    daily_file_syncs(args.sync_search_db).await,
                    "Failed calling sync_search_db",
                    true,
                    ErrorMsgFn::Error,
                );
            } else if args.sync_game_versions {
                handle_err_result(
                    game_version_sync(args.sync_game_versions).await,
                    "Failed calling sync_game_version",
                    true,
                    ErrorMsgFn::Error,
                );
            } else {
                handle_sync_call(&mod_dir, false).await;
            }
        }
        Commands::List(args) => {
            if args.game_versions.is_some() {
                let sorted_versions = sorted_game_versions().await;
                let filter_by = &args.game_versions.clone().unwrap_or("1.20".into());

                let versions: Vec<String> = sorted_versions
                    .into_iter()
                    .filter(|v| v.lower_contains(filter_by))
                    .collect();
                notice(
                    format!("[{}]", versions.join("], [").as_str()),
                    Some(Color::Yellow),
                    vec![],
                );
            } else {
                handle_err_result(
                    cmd_list(
                        &mod_dir,
                        args.updates,
                        false,
                        false,
                        args.export_args.columns.clone(),
                        Option::from(args.export_args.export_as.clone()),
                        args.export_args.file_path.clone(),
                    )
                    .await,
                    "Failed to display list",
                    true,
                    ErrorMsgFn::Error,
                );
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
            handle_err_result(
                download(args).await,
                "Failed download:",
                true,
                ErrorMsgFn::Error,
            );
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
                match install_missing_deps(&mod_dir, args.mod_ids.clone(), &mod_dir).await {
                    Ok(()) => {
                        handle_sync_call(&mod_dir, false).await;
                    }
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
        Commands::Misc {
            gen_auto_complete: Some(shell),
            ..
        } => {
            generate_completion(shell.clone());
        }
        Commands::Misc {
            one_click_setup: true,
            ..
        } => {
            one_click_setup();
        }
        Commands::Info(args) => {
            handle_err_result(
                info(args).await,
                "Failed to call Info:",
                true,
                ErrorMsgFn::Info,
            );
        }
        Commands::Search(args) => {
            handle_err_result(
                search(args).await,
                "Search failed:",
                true,
                ErrorMsgFn::Error,
            );
        }
        Commands::Modpack(cmds) => {
            parse_modpack_commands(cmds, &mod_dir).await;
        }
        Commands::Misc { .. } => {}
        Commands::RustiqueSelf(args) => {
            if args.check_updates {
                handle_err_result(
                    check_for_update(false, false).await,
                    "Update check failed:",
                    true,
                    ErrorMsgFn::Error,
                );
            }

            if args.update {
                handle_err_result(
                    update_manager::self_update_binary(args.force).await,
                    "Rustique update failed",
                    true,
                    ErrorMsgFn::Error,
                );
            }
        }

        Commands::Delete(args) => {
            if !args.mod_id.is_empty() {
                handle_err_result(
                    delete_cmd(&mod_dir, args.mod_id.clone(), args.mod_backups).await,
                    "Unable to delete mod(s)",
                    true,
                    ErrorMsgFn::Error,
                );
            }

            if args.all.is_some() {
                if let Some(which) = &args.all {
                    handle_err_result(
                        delete_all(&mod_dir, which).await,
                        &format!("Unable to delete all mod(s) in {}", mod_dir.display()),
                        true,
                        ErrorMsgFn::Error,
                    );
                }
            }
        }
    }
}

// Update this function to be async
async fn handle_sync_call(mod_dir: impl PathRef, quiet: bool) {
    match sync(mod_dir.as_ref(), quiet, vec![]).await {
        Ok(_) => {}
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
            println!(
                "# Save the above output to ~/.local/share/bash-completion/completions/Rustique"
            );
            println!(
                "# Or run: Rustique misc --gen-auto-complete bash > ~/.local/share/bash-completion/completions/Rustique"
            );
        }
        Shell::Zsh => {
            println!("# Save the above output to ~/.zsh/completion/_Rustique");
            println!(
                "# Or run: Rustique misc --gen-auto-complete zsh > ~/.zsh/completion/_Rustique"
            );
            println!("# Then add to your .zshrc: fpath=(~/.zsh/completion $fpath)");
        }
        Shell::Fish => {
            println!("# Save the above output to ~/.config/fish/completions/Rustique.fish");
            println!(
                "# Or run: Rustique misc --gen-auto-complete fish > ~/.config/fish/completions/Rustique.fish"
            );
        }
        Shell::PowerShell => {
            println!("# Save the above output to a file and source it in your PowerShell profile");
            println!("# Or run: Rustique misc --gen-auto-complete powershell > Rustique.ps1");
        }
        _ => {}
    }
}

fn one_click_setup() {
    let text = format!(
        "[Desktop Entry]
Type=Application
Name=VintageStory 1-click Mod Installer
Exec={} install %u
Terminal=true
StartupNotify=False
MimeType=x-scheme-handler/vintagestorymodinstall;",
        std::env::current_exe()
            .expect("Failed to get current executable path")
            .to_str()
            .unwrap()
    );
    let dir = home_dir()
        .expect("Failed to get home directory")
        .join(".local/share/applications/rustique.desktop");
    let filepath = match File::create("/usr/share/applications/rustique.desktop") {
        Ok(_) => "/usr/share/applications/rustique.desktop",
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => dir.to_str().unwrap(),
            _ => {
                error!("Failed to open desktop file: {}", e);
                dir.to_str().unwrap()
            }
        },
    };
    match File::create(filepath) {
        Ok(mut desktop_file) => match desktop_file.write_all(text.as_bytes()) {
            Ok(_) => {
                let _ = Command::new("xdg-mime")
                    .arg("default")
                    .arg("rustique.desktop")
                    .arg("x-scheme-handler/vintagestorymodinstall")
                    .status();
                notice(
                    "Desktop file for 1-click mod install created successfully.",
                    Some(Color::Green),
                    vec![],
                );
            }
            Err(e) => {
                error!("Failed to write to desktop file: {}", e);
            }
        },
        Err(e) => {
            error!("Failed to open desktop file: {}", e);
        }
    }
}
