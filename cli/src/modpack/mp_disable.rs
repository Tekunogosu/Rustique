use std::path::Path;

use comfy_table::{Attribute, Color};
use tracing::warn;
use core::aliases::{FileName, ModID};
use core::config::config_manager::get_config;
use core::information_utils::notice;
use core::symlink_manager::SymlinkManager;
use core::rustique_errors::RustiqueError;
use core::traits::ref_ext::PathRef;
use core::utils::extract_all_mods_metadata;

#[cfg(windows)]
use is_elevated::is_elevated;

#[cfg(windows)]
use std::process::exit;

pub async fn mp_disable(mpk_id: ModID, mod_dir: impl PathRef) -> Result<ModID, RustiqueError> {
   
    #[cfg(windows)]
      if !is_elevated() {
        notice("In order to disable modpacks, Rustique uses symlinks which require admin permissions on Windows. Please run Rustique with admin rights and try again.", Some(Color::Red), vec![Attribute::Bold]);
        exit(1);
    }
    
   
    let config = get_config().read().await;
    
    let mod_pack_dir = Path::new(&config.modpacks.modpack_dir).join("installed").join(&mpk_id);
    
    if !mod_pack_dir.exists() {
        return Err(RustiqueError::SimpleError("Modpack {} doesn't exist. Run 'Rustique modpack list' to view installed modpacks.".into()));
    }
    
    if !config.modpacks.enabled.contains(&mpk_id) {
        notice(format!("The requested modpack [{}] is not enabled, or you misstyped the ID", &mpk_id), Some(Color::Yellow), vec![Attribute::Bold]);
        return Err(RustiqueError::SimpleError("Modpack is not enabled".into()));
    }
    
    // check if requested modpack is enabled
    
    // if it is, get list of mods in that modpack, then remove them from the mod_dir
    
    let mods_in_pack: Vec<FileName> = extract_all_mods_metadata(mod_pack_dir, false).await?
        .keys().cloned().collect();
    
    // iterate through mods in the pack and try to remove the symlink
    
    for m in mods_in_pack {
        let p = mod_dir.as_ref().join(m);
        if SymlinkManager::exists(&p) {
            SymlinkManager::remove(&p)?;
        } else {
            warn!("Mod {} is no longer linked. Skipping..", p.display());
        }
    }
    
    Ok(mpk_id)
}