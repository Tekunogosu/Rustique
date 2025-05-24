use std::path::Path;
use comfy_table::{Attribute, Color};
use tracing::warn;
use crate::aliases::FileName;
use crate::commands::arg_structs::modpack_args::MPDisableArgs;
use crate::config::config_manager::get_config;
use crate::information_utils::notice;
use crate::modpack::symlink_manager::SymlinkManager;
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::PathRef;
use crate::utils::extract_all_mods_metadata;

pub async fn mp_disable(args: MPDisableArgs, mod_dir: impl PathRef) -> Result<String, RustiqueError> {
   
    let config = get_config().read().await;
    
    let mod_pack_dir = Path::new(&config.modpacks.modpack_dir).join("installed").join(&args.mpk_id);
    
    if !mod_pack_dir.exists() {
        return Err(RustiqueError::SimpleError("Modpack {} doesn't exist. Run 'Rustique modpack list' to view installed modpacks.".into()));
    }
    
    if !config.modpacks.enabled.contains(&args.mpk_id) {
        notice(format!("The requested modpack [{}] is not enabled, or you misstyped the ID", &args.mpk_id), Some(Color::Yellow), vec![Attribute::Bold]);
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
    
    Ok(args.mpk_id)
}