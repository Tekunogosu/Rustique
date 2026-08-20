use crate::config::config_manager::get_config;
use crate::utils::get_expanded_path;
use std::path::PathBuf;
#[cfg(any(windows, target_os = "macos"))]
use comfy_table::{Attribute, Color};
#[cfg(unix)]
use dirs::home_dir;
use tracing::info;

#[cfg(any(windows, target_os = "macos"))]
use crate::information_utils::notice;

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

        // no APPDATA means we can't guess. Hand back nothing and let the cli say something
        // useful about it, a library has no business killing the process
        RustiqueOptions { mod_dir: None }
    }

    // As of 1.21-pre*, the default location for Mac has changed to
    // $HOME/Library/Application Support/VintagestoryData/Mods
    #[cfg(unix)]
    pub fn unix() -> Self {
        // TODO: check if dir exists, if not check for the flatpack dir, throw error message if none are found
        if let Some(home) = home_dir() {

            #[cfg(target_os = "macos")]
            let mac_base = home
                .join("Library")
                .join("Application Support")
                .join("VintagestoryData")
                .join("Mods");

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


            #[cfg(target_os = "macos")]
            if mac_base.exists() {
                info!("Default mac mod dir found");
                options.mod_dir = Some(mac_base);
                return options;
            }

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

        // no home directory to work from. Same as above, report nothing and let the cli handle it
        RustiqueOptions { mod_dir: None }
    }

    // TODO: Finish mac migration to new config location
    #[cfg(target_os = "macos")]
    pub fn mac() -> Self {
        // Notify the user that the default location for Mac as of 1.21-pre* has been changed
        // Prompt if they would like to update the default location if they are using 1.21-pre+

        // check if new location exists
        // if yes, prompt user to update the default path and move the mods over
        // have option to disable message

        if let Some(home) = home_dir() {
            let new_default = home
                .join("Library")
                .join("Application Support")
                .join("VintagestoryData")
                .join("Mods");

            let old_default = home
                .join(".config")
                .join("VintagestoryData")
                .join("Mods");

            // if old default exists and IS NOT empty, prompt user
            // if its empty, check if new_default exist.
            // if new default does, use new default
            // if not, return old default, but let user know that 1.21 will use the new default

            let mut options = RustiqueOptions {
                mod_dir: Some(PathBuf::new())
            };

            // old exists, but is empty AND new exists, just use new location
            if old_default.exists()
                && old_default.read_dir().unwrap().count() <= 0
                && new_default.exists() {
                options.mod_dir = Some(new_default);
            } else if old_default.exists()
                && old_default.read_dir().unwrap().count() > 0
                && new_default.exists() {

                // let user know that the new location should be used and ask if they want to set the
                // default and move the mods over
                options.mod_dir = Some(old_default);

                notice(
                   "It looks like you are using the old default location for mods. As of Vintage Story 1.21, the new mac location is ~/Library/Application Support/VintagestoryData/Mods",
                   Some(Color::Yellow),
                   vec![Attribute::Bold],
                );
                notice(
                    "To update this run, rustique config set -m ~/Library/Application Support/VintagestoryData/Mods -- Note you will have to manually move your mods from the old location ~/.config/VintagestoryData/Mods",
                    Some(Color::Yellow),
                    vec![Attribute::Bold],
                );
            }



            return options;
        }

        RustiqueOptions { mod_dir: None }
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


}