use std::path::Path;
use comfy_table::{Attribute, Color};
use tracing::{error, info};
use owo_colors::OwoColorize;
use crate::commands::arg_structs::modpack_args::{ModpackCommands, ModpackSubCommands};
use crate::commands::info::info;
use crate::commands::list::new_list;
use crate::commands::update::update_mods;
use crate::config::config_manager::get_config;
use crate::information_utils::notice;
use crate::modpack::mp_create::{collect_mp_create_args, mp_create};
use crate::modpack::mp_disable::mp_disable;
use crate::modpack::mp_enable::mp_enable;
use crate::modpack::mp_install::mp_install;
use crate::traits::ref_ext::PathRef;

pub async fn parse_modpack_commands(commands: &ModpackCommands, mod_dir: impl PathRef) {
    match &commands.subcommand {
        ModpackSubCommands::Create(args) => {
            let parse_args = collect_mp_create_args(args);
            match mp_create(mod_dir, &mut parse_args.unwrap()).await {
                Ok(_) => {},
                Err(e) => {
                    error!("{}", e.to_string().red().bold());
                }
            }
        }
        ModpackSubCommands::Delete(args) => {}
        ModpackSubCommands::Install(args) => {
            match mp_install(args.clone()).await {
                Ok(installed) => {
                    // We update the config AFTER the installation so we know the lock on the config file is up
                    let mut config = get_config().write().await;
                    if !config.modpacks.disabled.contains(&installed) {
                        config.modpacks.disabled.push(installed);
                        match config.save(None) {
                            Ok(_) => {}
                            Err(e) => {
                                error!("{}", e.red().bold());
                            }
                        }
                    }
                                  }
                Err(e) => {
                    notice("Failed to install modpack. Maybe you have the wrong ID?", Some(Color::Red), vec![Attribute::Bold]);
                    // hide the error for cleaner UX
                    info!("{}", e.to_string().red().bold());
                }
            }
        }
        ModpackSubCommands::Enable(args) => {
            match mp_enable(args.clone(), mod_dir).await {
                Ok(enabled_pack) => {
                    let mut config = get_config().write().await;
                    config.modpacks.enabled.push(enabled_pack.clone());
                    config.modpacks.disabled.retain(|e| !e.eq_ignore_ascii_case(&enabled_pack));
                    match config.save(None) {
                        Ok(()) => {
                            notice(format!("Modpack: [{enabled_pack}] has been enabled!"), Some(Color::Green), vec![Attribute::Bold]);
                        }
                        Err(e) => { 
                            // If we fail to save, we should remove the symlinks
                            error!("{}", e.to_string().red().bold());
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to enable modpack.. :{}", e.to_string().red().bold());
                }
            }
        }
        ModpackSubCommands::Disable(args) => {
            match mp_disable(args.clone(), mod_dir).await {
                Ok(disabled_pack) => {
                    let mut config = get_config().write().await;
                    config.modpacks.enabled.retain(|m| !m.eq_ignore_ascii_case(&disabled_pack));
                    config.modpacks.disabled.push(disabled_pack.clone());
                    match config.save(None) {
                        Ok(()) => {
                           notice(format!("Modpack: [{disabled_pack}] has been disabled!"), Some(Color::Green), vec![Attribute::Bold]); 
                        } 
                        Err(e) => {
                           error!("{}", e.to_string().red().bold()); 
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to disable modpack.. :{}", e.to_string().red().bold());
                }
            }
        }
        ModpackSubCommands::List(args) => {
            let config = get_config().read().await;
            let packs_path = Path::new(&config.modpacks.modpack_dir).join("packs");
            match new_list(&packs_path, args.show_only_updates, true).await {
                Ok(()) => {}
                Err(e) => {
                    error!("{}", e.to_string().red().bold());
                }
            }
        }
        ModpackSubCommands::Update(args) => {
            todo!();
            // let config = get_config().read().await;
            // match update_mods(Path::new(&config.modpacks.modpack_dir).join("packs"), vec![args.mpk_id.clone()], false).await {
            //     Ok(()) => {}
            //     Err(e) => {
            //         error!("{}", e.to_string().red().bold());
            //     }
            // }
        }
        ModpackSubCommands::Info(args) => {
            match info(args).await {
                Ok(()) => {}
                Err(e) => {
                    error!("{}", e.to_string().red().bold());
                }
            }
        }
    }
}