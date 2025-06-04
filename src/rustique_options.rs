use std::path::PathBuf;
use crate::config::config_manager::get_config;
use crate::utils::get_expanded_path;

#[cfg(unix)]
use dirs::home_dir;

use tracing::info;

#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
use crate::information_utils::{rustique_message, RustiqueMessage};

#[cfg(windows)]
use crate::rustique_errors::RustiqueError;

#[cfg(windows)]
use comfy_table::{Attribute, CellAlignment};

#[cfg(windows)]
use std::env;

#[cfg(windows)]
use crate::utils::extract_all_mods_metadata;

#[cfg(windows)]
use tracing::error;

#[cfg(windows)]
use crate::commands::delete::iterate_and_move_zip;

#[cfg(windows)]
use is_elevated::is_elevated;
#[cfg(windows)]
use crate::modpack::mp_disable::mp_disable;
#[cfg(windows)]
use crate::modpack::mp_enable::mp_enable;

#[cfg(windows)]
use comfy_table::Color;

#[cfg(windows)]
use crate::information_utils::{notice, CellData};

#[cfg(windows)]
use std::process::exit;

#[derive(Clone, Debug)]
pub struct RustiqueOptions {
    pub mod_dir: Option<PathBuf>,
}

impl RustiqueOptions {
    pub fn default() -> Self {
        #[cfg(windows)]
        return Self::windows();

        #[cfg(unix)]
        return Self::unix();
    }

    #[cfg(windows)]
    pub fn windows() -> Self {
        if let Some(path) = std::env::var_os("APPDATA") {
            return RustiqueOptions {
                mod_dir: Some(PathBuf::from(path).join("VintagestoryData").join("Mods")),
            }
        }
        panic!("Unable to determine default mods directory");
    }

    // this also works for mac
    #[cfg(unix)]
    pub fn unix() -> Self {
        // TODO: check if dir exists, if not check for the flatpack dir, throw error message if none are found
        if let Some(home) = home_dir() {
            let base =  home
                .join(".config")
                .join("VintagestoryData")
                .join("Mods");

            let flatpak = home
                .join(".var")
                .join("app")
                .join("at.vintagestory.VintageStory")
                .join("config")
                .join("VintagestoryData")
                .join("Mods");

            let mut options = RustiqueOptions {
                mod_dir: Some(PathBuf::new())
            };

            if base.exists() {
                info!("normal mod dir found");
                options.mod_dir = Some(base);
            } else if flatpak.exists() {
                info!("flatpak mod dir found");
                options.mod_dir = Some(flatpak);
            } else {
                info!("Rustique was unable to find the default mod dir. Using empty dir for now.");
                options.mod_dir = None;
            }

            return options
        }
        panic!("Unable to determine user's home directory, do you have permissions??");
    }

pub async fn get_mod_path(&self) -> PathBuf {
        let default_path = self.mod_dir.clone().unwrap_or_default();
        let config = get_config().read().await;
        let config_mod_dir = PathBuf::from(&config.mod_dir);

        if default_path.as_path().eq(get_expanded_path(config_mod_dir.clone()).as_path()) {
            default_path
        } else {
            config_mod_dir
        }
    }

    #[cfg(windows)]
    pub async fn check_old_default_windows() -> Result<(), RustiqueError> {

        // check what is currently the default in the config - if it exists.
        // check if there are any mods in the old default location
        // prompt the user if they would like to move the mods to the preferred default location
        // if yes, move the mods and update the config file
        // if no, don't move mods, ask if they would like to silence the check so they are not bugged again in the future

        let config = get_config().read().await;
        let mod_dir = config.mod_dir.clone();
        drop(config);

        if let Some(app_data) = env::var_os("APPDATA") {
            let old_default = Path::new(&app_data).join("Vintagestory").join("Mods");
            if !old_default.exists() {
                // Means game is not installed or user setup their own path, so just exit.
                return Ok(());
            }

            let new_default = Path::new(&app_data).join("VintagestoryData").join("Mods");

            // path is valid, check if any mods exist. This will only look for the presence of .zip mod files
            let mod_metadata = extract_all_mods_metadata(&old_default, false).await?;

            // First check if there are any mods present and prompt the user.
            // if they choose to do the swittch, check for enabled modpacks that need to be disabled
            // move all .zips over.
            // enable any modpacks that were enabled before
            let mut can_proceed = false;
            if !mod_metadata.is_empty() {
                // prompt user if we can proceed
                rustique_message(RustiqueMessage {
                    header: Some(CellData::new("Attention!".into(), Some(Color::Yellow), vec![Attribute::Bold], Some(CellAlignment::Center))),
                    message: vec![
                        CellData::new("Currently, you are using the old default location for mods, which is:".into(), Some(Color::Yellow), vec![], Some(CellAlignment::Center)),
                        CellData::new(old_default.to_string_lossy().to_string(), Some(Color::Cyan), vec![Attribute::Bold], Some(CellAlignment::Center)),
                        CellData::blank(),
                        CellData::new("The correct default should be:".into(), Some(Color::Yellow), vec![], Some(CellAlignment::Center)),
                        CellData::new(new_default.to_string_lossy().to_string(), Some(Color::Cyan), vec![], Some(CellAlignment::Center)),
                        CellData::blank(),
                        CellData::new("Rustique can update this location and move your mods. This changes only the default mod location that Rustique uses and WILL NOT affect gameplay.".into(), Some(Color::Yellow), vec![], Some(CellAlignment::Center)),
                    ],
                });

                loop {
                    print!("Would you like Rustique to update your default path? [Y/N]: ");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).expect("Unable to read input, try again.");
                    match  input.trim().to_lowercase().as_str() {
                        "y" | "yes" => {
                            can_proceed = true;
                            break
                        },
                        "n" | "no" => break,
                        _ => println!("Please enter 'y' or 'yes', 'n' or 'no'"),
                    }
                }

                if !can_proceed {
                    //
                    notice("Ok, Rustique will not update your mod location. To prevent Rustique from checking again, use the following command:", Some(Color::Green), vec![]);
                    notice("Rustique config set --update-default-windows-loc false", Some(Color::Cyan), vec![Attribute::Bold]);
                    return Ok(());
                }

                // User has given permission to update the location.. do the stuff
                // check for any enabled modpacks and disable them
                let enabled_modpacks: Vec<String> = {
                    let config = get_config().read().await;
                    config.modpacks.enabled.clone()
                };

                if !enabled_modpacks.is_empty() {

                    if !is_elevated() {
                        notice("You have modpacks enabled. Rustique will need admin right to disable, then enable your modpacks.", Some(Color::Yellow), vec![Attribute::Bold]);
                        exit(0);
                    }
                    info!("User running with admin right, continuing");

                    for mpk_id in &enabled_modpacks {
                        match mp_disable(mpk_id.clone(), &old_default).await {
                            Ok(modpack) => {
                               let mut config = get_config().write().await;
                                config.modpacks.enabled.retain(|m| !m.eq_ignore_ascii_case(&modpack));
                                config.modpacks.disabled.push(modpack.clone());
                                config.save(None)?;
                                info!("disabled {modpack}")
                            }
                            Err(e) => {
                                error!("Rustique was enable to disable your modpack {mpk_id} and cannot continue: {e}");
                                exit(1);
                            }
                        }
                    }
                }

                // iterate though all the .zips and move them to the new location
                let mut mods = tokio::fs::read_dir(&old_default).await?;

                if let Err(e) =  iterate_and_move_zip(&mut mods, &new_default, true).await {
                    notice(format!("Rustique ran into errors while attempting to move your mods. You will need to move them manually, then use the following command to reset the default dir to the new one: {e}"), Some(Color::Red), vec![Attribute::Bold]);
                    notice("Rustique config del -m:", Some(Color::Cyan), vec![Attribute::Bold]);
                    exit(1);
                }

                // files have been moved, update the default location and enable any modpacks


                if Path::new(&mod_dir) == old_default {
                    let mut config = get_config().write().await;
                    config.mod_dir = new_default.to_string_lossy().to_string();
                    config.save(None)?;
                    info!("Updated mod_dir in config to new path {}", new_default.display());
                }


                for mpk_id in &enabled_modpacks {
                    match mp_enable(mpk_id.clone(), &new_default, true).await {
                        Ok(modpack) => {
                            let mut config = get_config().write().await;
                            info!("Enabled {modpack}");
                            config.modpacks.enabled.push(modpack.clone());
                            config.modpacks.disabled.retain(|m| !m.eq_ignore_ascii_case(&modpack));
                            config.save(None)?;
                            drop(config);
                        }
                        Err(e) => {
                            notice(format!("Rustique was enable to enable your modpack {mpk_id}, try using Rustique enable {mpk_id} instead: {e}"), Some(Color::Red), vec![Attribute::Bold]);
                        }
                    }
                }
            }
        } else {
            info!("Unable to reade the appdata folder. This should not happen and will cause errors with rustique");
            return Ok(())
        }


        Ok(())
    }
}