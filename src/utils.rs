use std::path::PathBuf;
use dirs::home_dir;
use serde::{Deserialize, Serialize};

pub const API_BASE: &str = "https://mods.vintagestory.at/api";

pub mod api {
    use super::API_BASE;

    pub fn uri(endpoint: &str) -> String {
        format!("{}/{}", API_BASE, endpoint)
    }

    pub fn mods() -> String {
        uri("mods")
    }

    pub fn get_mod(modid: &str) -> String {
        uri(&format!("mods/{}", modid))
    }
}

pub const DEFAULT_MOD_DIR_LINUX: &str = ".config/VintagestoryData/Mods";

// TODO: Check on location for Vintage Story Flatpak

// TODO: Needs validation and checking on windows to confirm
pub const DEFAULT_MOD_DIR_WINDOWS: &str = "%APPDATA%\\Vintagestory\\Mods";

pub struct ModOptions {
    pub moddir: PathBuf,
}

impl ModOptions {
    pub fn default() -> Self {
        if cfg!(target_os = "windows") {
            Self::windows()
        } else {
            Self::linux()
        }
    }

    pub fn windows() -> Self {
        if let Some(path) = std::env::var_os("APPDATA") {
            return ModOptions {
                moddir: PathBuf::from(path).join("Vintagestory").join("Mods"),
            }
        }
        panic!("Unable to determine default mods directory");
    }

    pub fn linux() -> Self {
        if let Some(home) = home_dir() {
            return ModOptions {
                moddir: home.join(".config").join("VintagestoryData").join("Mods"),
            };
        }
        panic!("Unable to determine user's home directory, do you have permissions??");
    }
}

pub fn get_case_insensitive<'a>(obj: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    if let Some(obj) = obj.as_object() {
        obj.iter()
            .find(|(k, _)|k.to_lowercase() == key.to_lowercase())
            .map(|(_, v)| v)
    } else {
        None
    }
}