

// Delete a modpack. if the pack is enabled, return an error stating they need to disable it first,
// this prevents people from unintentionally deleting an active modpack.

use std::collections::HashMap;
use std::path::Path;
use tracing::info;
use core::aliases::{ModFileName, ModID};
use core::api::api_structs::ModInfo;
use core::config::config_manager::get_config;
use core::rustique_errors::RustiqueError;
use core::traits::ref_ext::PathRef;
use core::utils::{delete_file, extract_all_mods_metadata};

pub async fn delete_mpk_cmd(mpk_id: ModID) -> Result<ModID, RustiqueError> {
    // verify the modpack is installed
    // verify its not enable
    // if yes, delete the .zip from mypacks
    // check if mods dir exist and delete it from installed
    // update config to 

    let (p, mut disabled, enabled_contains, disabled_contains) = {
        let config = get_config().read().await;
        (
            config.modpacks.modpack_dir.clone(),
            config.modpacks.disabled.clone(),
            config.modpacks.enabled.contains(&mpk_id),
            config.modpacks.disabled.contains(&mpk_id),
        )
    };

    if  enabled_contains {
        return Err(RustiqueError::SimpleError(format!("{mpk_id} is currently enabled! Disable it first before attempting to delete it.")));
    }
    
    if !disabled_contains {
        return Err(RustiqueError::SimpleError(format!("{mpk_id} is not installed. Check your spelling and try again.")));
    }
   
    
    let base_dir = Path::new(&p);
    
    if !base_dir.exists() {
        return Err(RustiqueError::SimpleError("Your modpacks directory does not exist! 'Run Rustique config list' to see what its set to.".into()));
    }
    
    let mpk_mods_dir = base_dir.join("installed").join(&mpk_id);
    if mpk_mods_dir.exists() {
        tokio::fs::remove_dir_all(&mpk_mods_dir).await?;
    }
    
    let packs = extract_all_mods_metadata(&base_dir.join("packs"), false).await?;
  
    match check_and_remove(&mpk_id, packs, &base_dir.join("packs")).await {
        Ok(pack_id) => {
            info!("packs retain {pack_id}");
            disabled.retain(|m| m != pack_id);
        }
        Err(e) => {
            info!("{e}");
        }
    }
    
    
   let my_packs = extract_all_mods_metadata(&base_dir.join("mypacks"), false).await?;
   
     match check_and_remove(&mpk_id, my_packs, &base_dir.join("mypacks")).await {
         Ok(pack_id) => {
             info!("mypacks retain {pack_id}");
             disabled.retain(|m| m != pack_id);
         }
         Err(e) => {
            info!("{e}");   
         }
    }
   
    Ok(mpk_id)
}

async fn check_and_remove(mpk_id: &ModID, mpk_data: HashMap<ModFileName, ModInfo>, mpk_mods_dir: impl PathRef) -> Result<&ModID, RustiqueError> {
    for (filename, mod_info) in mpk_data {
        if &mod_info.mod_id == mpk_id {
            info!("deleting {}", filename);
            delete_file(&mpk_mods_dir.as_ref().join(&filename)).await?;
            return Ok(mpk_id);
        }
    }
    Err(RustiqueError::SimpleError("Modpack not found".to_string()))
}