use std::sync::{RwLock, OnceLock};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::exit;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use crate::rustique_errors::RustiqueError;
use crate::utils::{get_expanded_path, RustiqueOptions};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    // this sets the default mod dir so you don't have to type -m everytime
    pub mod_dir: String,
    // this tells rustique which versions of the game to download mods for.
    // It will download mods up to this version and not over
    pub pinned_game_version: String,
    // automatically zips mod folders that are unzipped during the sync process
    pub zip_mod_files: bool,
    // create a backup of each mod before its updated.
    pub backup_mods: bool,

    pub mod_pack: ModPack,
    pub alias: Vec<AliasConfig>,
    pub table: TableConfig,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ModPack {
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AliasConfig {
    pub name: String,
    pub mod_dir: String,
    pub pinned_game_version: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TableConfig {
    headers: HashMap<String, Vec<String>>,
    cells: HashMap<String, Vec<String>>,
}

pub const CONFIG_DEFAULT_DIR: &str = "~/.config/rustique";

impl Default for Config {
    fn default() -> Self {
        Self {
            mod_dir: RustiqueOptions::default().mod_dir.unwrap().to_string_lossy().to_string(),
            pinned_game_version: "".to_string(), // if its empty then get the latest
            zip_mod_files: false,
            backup_mods: false,
            mod_pack: ModPack {},
            alias: vec![],
            table: TableConfig {
                headers: HashMap::new(),
                cells: HashMap::new(),
            },
        }
    }
}

impl Config {
    pub fn new(config_dir: Option<PathBuf>) -> Result<Config, RustiqueError> {

        let config_path = config_dir.unwrap_or_else(|| get_expanded_path(PathBuf::from(CONFIG_DEFAULT_DIR)));

        if !config_path.exists() {
            fs::create_dir_all(&config_path).map_err(|e| {
                RustiqueError::ConfigFileError(format!("Failed to create config directory: {}", e.to_string()))
            })?;
        }

        let config_file_path = config_path.join("config.toml");

        if !config_file_path.exists() {
            let default_config = Self::default();
            let toml_content = toml::to_string_pretty(&default_config).map_err(|e| {
                RustiqueError::ConfigFileError(format!("Failed to serialize default config: {}", e.to_string()))
            })?;

            let mut file = File::create(&config_file_path).map_err(|e| {
                RustiqueError::ConfigFileError(format!("Failed to create config file at: {}", e.to_string()))
            })?;

            file.write_all(toml_content.as_bytes()).map_err(|err| {
                RustiqueError::ConfigFileError(format!("Failed writing config file: {}", err.to_string()))
            })?;

            eprintln!("Successfully created config file: {}", config_file_path.display());
            return Ok(default_config);
        };

        // if config exists load and parse it
        let mut file = File::open(&config_file_path)
            .map_err(|e| RustiqueError::ConfigFileError(format!("Failed to open config file: {}", e.to_string())))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            RustiqueError::ConfigFileError(format!("Failed to read config file: {}", e.to_string()))
        })?;

        match toml::from_str::<Config>(&contents) {
            Ok(config) => Ok(config),
            Err(e) => {
                eprintln!("Failed to parse config: {}", e.to_string());
                eprintln!("Using default config");
                Ok(Config::default())
            }
        }
    }

    pub fn save(&self, config_dir: Option<PathBuf>) -> Result<(), RustiqueError> {
        let config_path = config_dir.unwrap_or_else(|| get_expanded_path(PathBuf::from(CONFIG_DEFAULT_DIR)));
        let config_file_path  = config_path.join("config.toml");

        let toml_content = toml::to_string_pretty(self).map_err(|e| {
            RustiqueError::ConfigFileError(format!("Failed to serialize config: {}", e.to_string()))
        })?;

        File::create(&config_file_path)
            .map_err(|e| RustiqueError::ConfigFileError(format!("Failed to create config file: {}", e.to_string())))?
            .write_all(toml_content.as_bytes())
            .map_err(|e| RustiqueError::ConfigFileError(format!("Failed to write config file: {}", e.to_string())))?;

        Ok(())
    }
}

static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();

// Initiate the CONFIG in the main file so its ready everywhere else
pub fn init_config(config_dir: Option<PathBuf>) -> Result<(), RustiqueError> {
    let config = Config::new(config_dir)?;

    if CONFIG.set(RwLock::new(config)).is_err() {
        return Err(RustiqueError::ConfigFileError("Config has already been initialized".to_string()));
    }

    Ok(())
}

pub fn get_config() -> &'static RwLock<Config> {
    CONFIG.get_or_init(|| RwLock::new(Config::new(None).expect("Config has not been initialized")))
}