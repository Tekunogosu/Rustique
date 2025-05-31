use crate::rustique_errors::RustiqueError;
use crate::utils::RustiqueOptions;
use chrono::Local;
use comfy_table::{Attribute, CellAlignment, Color};
use dirs::home_dir;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use crate::aliases::ModVersion;
use crate::config::config_structs::Tables;
use crate::information_utils::{rustique_message, CellData, RustiqueMessage};
use crate::traits::ref_ext::PathRef;

#[derive(Deserialize, Serialize, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct Config {
    /// this sets the default mod dir so you don't have to type -m everytime
    #[serde(default)]
    pub mod_dir: String,
    // this tells rustique which versions of the game to download mods for.
    // It will download mods up to this version and not over
    #[serde(default)]
    pub pinned_game_version: String,
    // automatically zips mod folders that are unzipped during the sync process
    #[serde(default)]
    pub zip_mod_files: bool,
    // create a bacokup of each mod before its updated.
    #[serde(default)]
    pub backup_mods: bool,

    // location for the mod backups
    // default ~/.config/rustique/backups
    #[serde(default)]
    pub backup_mods_dir: String,

    
    
    // Shows the "<operation> completed: " text after a command finishes
    #[serde(default)]
    pub show_execution_time: bool,

    #[serde(default)]
    pub notify_of_unzipped_mods: bool,
    
    #[serde(default)]
    pub game_download_dir: String,
    
    #[serde(default)]
    pub check_for_updates: bool,

    #[serde(default)]
    pub modpacks: ModPacks,
    
    
    #[serde(default)]
    pub pkg: Vec<Package>,
   
    #[serde(default = "default_sync_time")]
    pub sync_latest_game_version_file_every: i64,
    
    #[serde(default = "default_sync_time")]
    pub sync_mod_search_file_every: i64,

    #[serde(default)]
    pub table: Tables,
    
    
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModPacks {
    #[serde(default)]
    pub modpack_dir: String,
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

// Manually set the default since we need the default modpack_dir to be set to something specific
// Otherwise its set to a blank string which will make modpacks installs fail.
impl Default for ModPacks {
    fn default() -> Self {
        Self {
            modpack_dir: Config::get_path().join("modpacks").to_string_lossy().to_string(),
            enabled: vec![],
            disabled: vec![],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Package {
    pub mod_id: String,
    #[serde(default)]
    pub pinned_version: Option<ModVersion>,
}

fn default_sync_time() -> i64 {
    24
}

impl Config {
    pub fn get_path() -> PathBuf {
        if cfg!(target_os = "windows") {
            if let Some(w_path) = std::env::var_os("APPDATA") {
                PathBuf::from(w_path).join("rustique")
            } else {
                PathBuf::from("../..").join("rustique")
            }
        } else if let Some(u_path) = home_dir() {
            u_path.join(".config").join("rustique")
        } else {
            PathBuf::from("../..").join("rustique")
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        // let backup_mods_dir = get_expanded_path(PathBuf::from(CONFIG_DEFAULT_DIR).join("mod_backups"));
        let backup_mods_dir = Self::get_path().join("mod_backups");
        let modpack_dir = Self::get_path().join("modpacks");
        
        match Self::setup_modpack_dir("modpacks") {
            Ok(_) => {},
            Err(e) => {
                warn!("Failed to setup modpack dir: {}", e);
            }
        }
    
        info!("modpack_dir {}", modpack_dir.display());

        Self {
            mod_dir: RustiqueOptions::default()
                .mod_dir
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            pinned_game_version: String::new(), // if its empty then get the latest
            zip_mod_files: false,
            backup_mods: false,
            backup_mods_dir: backup_mods_dir.to_string_lossy().to_string(),
            show_execution_time: true,
            notify_of_unzipped_mods: false,
            game_download_dir: dirs::download_dir().unwrap_or_default().to_string_lossy().to_string(),
            sync_latest_game_version_file_every: 24,
            sync_mod_search_file_every: 24,
            pkg: Vec::default(),
            table: Tables::with_defaults(),
            modpacks: ModPacks::default(),
            check_for_updates: true,
        }
    }
}

impl Config {
    pub fn new(config_dir: Option<PathBuf>) -> Result<Config, RustiqueError> {
        let config_path = config_dir.unwrap_or_else(Self::get_path);

        if !config_path.exists() {
            fs::create_dir_all(&config_path).map_err(|e| {
                RustiqueError::ConfigFileError(format!("Failed to create config directory: {e}"))
            })?;
        }

        let config_file_path = config_path.join("config.toml");

        if !config_file_path.exists() {
            let default_config = Self::default();
            let toml_content = toml::to_string_pretty(&default_config).map_err(|e| {
                RustiqueError::ConfigFileError(format!(
                    "Failed to serialize default config: {e}"
                ))
            })?;

            let mut file = File::create(&config_file_path).map_err(|e| {
                RustiqueError::ConfigFileError(format!(
                    "Failed to create config file at: {e}"
                ))
            })?;

            file.write_all(toml_content.as_bytes()).map_err(|e| {
                RustiqueError::ConfigFileError(format!(
                    "Failed writing config file: {e}"
                ))
            })?;

            println!(
                "{} {}",
                "Successfully created config file: ".green(),
                config_file_path.display().to_string().bright_yellow()
            );
            return Ok(default_config);
        }

        // make sure the modpack_dir is setup early
        Self::setup_modpack_dir("modpacks")?;

        // if config exists load and parse it
        let mut file = File::open(&config_file_path).map_err(|e| {
            RustiqueError::ConfigFileError(format!("Failed to open config file: {e}"))
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            RustiqueError::ConfigFileError(format!("Failed to read config file: {e}"))
        })?;
       
        

        match toml::from_str::<Config>(&contents) {
            Ok(config) => Ok(config),
            Err(e) => {
                backup_config(&config_file_path, Some(e.to_string()))?;

                // write the default
                let config = Config::default();
                config.save(Option::from(Config::get_path()))?;

                Ok(config)
            }
        }
    }
    
    pub fn setup_modpack_dir(modpack_dir: impl PathRef) -> Result<(), RustiqueError> {
        let modpack_dir = Self::get_path().join(modpack_dir);
        // create the modpack directory if it hasn't been created
        debug!("Checking if {} exists", modpack_dir.to_string_lossy());
        if !&modpack_dir.exists() {
            info!("Created modpack directory");

            for dir in ["installed", "packs", "mypacks"] {
                info!("creating modpacks/{dir}");
                let d = &modpack_dir.join(dir);
                fs::create_dir_all(d)
                    .map_err(|e| RustiqueError::SimpleError(format!("Failed to create {}: {}", d.to_string_lossy(), e)))?;
            }
        }


        Ok(())
    }

    pub fn save(&self, config_dir: Option<PathBuf>) -> Result<(), RustiqueError> {
        let config_path = config_dir.unwrap_or_else(Self::get_path);
        let config_file_path = config_path.join("config.toml");

        let toml_content = toml::to_string_pretty(self).map_err(|e| {
            RustiqueError::ConfigFileError(format!("Failed to serialize config: {e}"))
        })?;

        File::create(&config_file_path)
            .map_err(|e| {
                RustiqueError::ConfigFileError(format!("Failed to create config file: {e}"))
            })?
            .write_all(toml_content.as_bytes())
            .map_err(|e| {
                RustiqueError::ConfigFileError(format!("Failed to write config file: {e}"))
            })?;

        Ok(())
    }
}

pub fn backup_config(config_path: impl PathRef, message: Option<String>) -> Result<(), RustiqueError> {
    let config_path = config_path.as_ref();
    if config_path.exists() {
        let back_name = format!("toml.bak-{}", Local::now().format("%Y%m%d_%H%M%S"));
        let backup_path = config_path.with_extension(&back_name);

        let h1 = CellData::new(
            "Rustique has discovered an error with your config.toml file".to_string(),
            Some(Color::Magenta),
            vec![Attribute::Bold],
            None,
        );

        let m1 = CellData::new(
            "Your old config has been backed up to the following location:".to_string(),
            Some(Color::Yellow),
            vec![],
            None,
        );

        let m2 = CellData::new(
            format!("{}", config_path.with_extension(&back_name).display()),
            Some(Color::Green),
            vec![Attribute::Bold],
            None,
        );

        let m3 = CellData::new(
          "A new config has been written using default values. You will need to set your configuration options again.".to_string(),
          Some(Color::Yellow),
          vec![],None,
        );

        let m4 = CellData::new(String::new(), None, vec![], None);
        let m5 = CellData::new(
            message.unwrap_or_default(),
            Some(Color::Red),
            vec![Attribute::Bold, Attribute::Italic],
            Some(CellAlignment::Left),
        );

        rustique_message(RustiqueMessage {
            header: Some(h1),
            message: vec![m1, m2, m3, m4, m5],
        });

        fs::copy(config_path, &backup_path)?;
    }

    Ok(())
}

static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();

// Initiate the CONFIG in the main file so its ready everywhere else
pub fn init_config(config_dir: Option<PathBuf>) -> Result<(), RustiqueError> {
    let config = Config::new(config_dir)?;

    if CONFIG.set(RwLock::new(config)).is_err() {
        return Err(RustiqueError::ConfigFileError(
            "Config has already been initialized".to_string(),
        ));
    }

    Ok(())
}

pub fn get_config() -> &'static RwLock<Config> {
    info!("get_config() called");
    CONFIG.get_or_init(|| RwLock::new(Config::new(None).expect("Config has not been initialized")))
}

