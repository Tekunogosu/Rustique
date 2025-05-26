
// mp_install calls normal install, except it gathers a list of all the required mods before hand 
// and sets the install location to be the modpack specific folder

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::exit;
use std::time::Instant;
use comfy_table::{Attribute, Color};
use owo_colors::OwoColorize;
use tracing::{debug, info, warn};
use crate::aliases::{ModID, ModVersion};
use crate::api::api_structs::ModInfo;
use crate::api::client::ApiClient;
use crate::api::download::download_requested_mods;
use crate::commands::sync::sync;
use crate::config::config_manager::{get_config, Package};
use crate::consts::FILE_MODINFO_JSON;
use crate::information_utils::{command_output, display_table, elapsed_footer, notice};
use crate::install_manager::{install_manager, Install};
use crate::rustique_errors::RustiqueError;
use crate::utils::extract_zip_metadata;
use crate::version_management::{parse_download_url_from_version, parse_latest_version, parse_pinned_version};


pub fn check_if_mp_enabled(mp_id: &ModID, array: &Vec<String>) {
    if array.contains(mp_id) {
        notice(format!("{} {}", mp_id, "is currently enabled!. Disable it first then try again. "), Some(Color::Yellow), vec![]);
        exit(1);
    }
}

pub async fn mp_install(mp_id: ModID, mp_version: Option<ModVersion>) -> Result<String, RustiqueError> {
    let start_time = Instant::now();
    // installing the modpack with this function will do the following:
    // Save the modpack.zip (the modpack from the mods website) to modpacks/packs
    // Once the modpack is installed, it will download all the mods associated with the modpack  
    // to the location [modpacks/installed/modpack_id/*] 
    let config = get_config().read().await;
    
    check_if_mp_enabled(&mp_id, &config.modpacks.enabled);
        
    let client = ApiClient::new();
    
    let mod_info = client.fetch_mod(&mp_id).await?;

    let installed_dir = Path::new(&config.modpacks.modpack_dir).join("installed");
    
    let (version, download_url, _) = if let Some(pin_version) = mp_version  {
        let pkg = Package {
            mod_id: mp_id.clone(),
            pinned_version: Some(pin_version),
        };
        parse_pinned_version(&mod_info.mod_json.releases, &pkg, String::new())
    } else {
        parse_latest_version(&mod_info.mod_json.releases)
    };
    
    
    let install_modpack = Install {
            mod_id: mod_info.mod_json.mod_id.clone().to_string(),
            mod_name: mod_info.mod_json.name.clone().unwrap_or_default(),
            version_to_install: version,
            download_url,
            current_file_path: None,
    };
    
    notice(format!("Downloading Modpack {mp_id}..."), Some(Color::Green), vec![]);
    
    // download the modpack first, then install the dependencies
    let packs_dir = Path::new(&config.modpacks.modpack_dir).join("packs");
    let Some(modpack) = download_requested_mods(&packs_dir, &mut vec![install_modpack], &client).await?.into_iter().next() else {
            return Err(RustiqueError::SimpleError("Modpack download failure..".into()));
        };

    if let Some(modpack_packs_path) = modpack.installed_file_path {
        // Modpack is just a normal mod but we treat it differently when used with modpack
        let modpack_info = extract_zip_metadata::<ModInfo>(&modpack_packs_path, FILE_MODINFO_JSON).inspect_err(|_| {
            notice(format!("The requested modpack has a malformed {FILE_MODINFO_JSON} file and Rustique is unable to parse it."), Some(Color::Red), vec![Attribute::Bold]);
        })?;
        
        // do another check if the IDs are different, user might have installed using the numerical ID
        if !modpack_info.mod_id.eq_ignore_ascii_case(&mp_id) {
            check_if_mp_enabled(&modpack_info.mod_id, &config.modpacks.enabled);
        }

        // The modpack is installed to the correct place, install all dependencies
        let modpack_mod_path = installed_dir.join(&modpack_info.mod_id);

        if !modpack_mod_path.exists() {
            info!("Created {modpack_mod_path:?}");
            fs::create_dir_all(&modpack_mod_path)?;
        }

        // grab the mod ids from the modpack
        let mods = modpack_info.dependencies.keys().cloned().collect();
        let mod_pkgs: Vec<Package> = modpack_info.dependencies.iter().map(|(id, version)| Package {
            mod_id: id.clone(),
            pinned_version: Some(version.clone()),
        }).collect();
        
        info!("MODS: {mods:?}");
        let deps = client.fetch_mods_parallel(mods).await?;
        
        let install_mp_mods: Vec<Install> = deps.iter().filter_map(|(mod_id, mod_api)| {
            // grab the mod from the modpack so we can actually download the correct version
            if let Some((mp_mod_id,mp_mod_version)) = modpack_info.dependencies.iter().find(|(dep_mod_id, _) |dep_mod_id.eq(&mod_id)) {
                let download_url = match parse_download_url_from_version(&mod_api.mod_json.releases, mp_mod_version) {
                    Ok(download_url) => download_url,
                    Err(e) => {
                        warn!("Rustique can't download {}: {}", mp_mod_id.red(), e.red());
                        return None;
                    }
                };
                
                Some(Install {
                    mod_id: mod_id.clone(),
                    mod_name: mod_api.mod_json.name.clone().unwrap_or_default(),
                    version_to_install: mp_mod_version.clone(),
                    download_url,
                    current_file_path: None,
                })
            } else {
                None
            }
        }).collect();
        
        debug!("Need to download {install_mp_mods:#?}");

        let installed = install_manager(&modpack_mod_path, install_mp_mods, HashMap::new()).await?;
       
        // Mod saved successfully, add it to the disabled mods so we know its installed
        
        debug!("Successfully installed {installed:#?}");
        
        sync(packs_dir, false, vec![]).await?;
        sync(modpack_mod_path, false, mod_pkgs).await?;
        
        
        display_table(vec![command_output("Successfully installed Modpack:", modpack.mod_name)], None);
        elapsed_footer(start_time, "Modpack Install");
        
        
        
        return Ok(modpack_info.mod_id.clone());
    }
    
    Err(RustiqueError::SimpleError("Unable to find installed modpack".into()))
}