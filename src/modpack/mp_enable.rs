

// Enabling a modpack is a bit of work. First it mvs the existing default mod directory to the rustique backup location,
// then create a symlink to the modpack. Only one modpack can be enabled at a time

use std::path::Path;
use comfy_table::{Attribute, Color};
use comfy_table::presets::UTF8_HORIZONTAL_ONLY;
use crate::aliases::FileName;
use crate::commands::arg_structs::modpack_args::MPEnableArgs;
use crate::config::config_manager::get_config;
use crate::information_utils::{display_table, notice, CellData};
use crate::modpack::symlink_manager::SymlinkManager;
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::PathRef;
use crate::utils::extract_all_mods_metadata;

pub async fn mp_enable(args: MPEnableArgs, mod_dir: impl PathRef) -> Result<String, RustiqueError> {
    let config = get_config().read().await;
    
    let mod_pack_install_dir = Path::new(&config.modpacks.modpack_dir).join("installed").join(&args.mpk_id);
    
    if !mod_pack_install_dir.exists() {
        return Err(RustiqueError::SimpleError("Modpack {} doesn't exist. Run 'Rustique modpack list' to view installed modpacks.".into()));
    }
    
    // check if a modpack already exists
    // if so, notify the user and tell them to either disable the current one OR use modpack enable -f to force the use and warn about using multiple
    
    // Is it already enabled?
    if config.modpacks.enabled.contains(&args.mpk_id) {
        notice(format!("Modpack: [{}] is already enabled. Did you mean to disable it?", &args.mpk_id), Some(Color::Yellow), vec![Attribute::Bold]);
        return Err(RustiqueError::SimpleError("Modpack already enabled".into()));
    }
    
    // Is it even installed??
    if !config.modpacks.disabled.contains(&args.mpk_id) {
        notice(format!("Modpack: [{}] is not installed! Use [Rustique modpack install {}] to install it first.", &args.mpk_id, &args.mpk_id), Some(Color::Yellow), vec![Attribute::Bold]);
        return Err(RustiqueError::SimpleError("Modpack needs to be installed first".into()))
    }
    
    // Is anything else enabled?
    if !config.modpacks.enabled.is_empty() && !&args.force {
        
        display_table(vec![
            (CellData::new("You already have the following modpack(s) enabled: ".into(), Some(Color::Yellow), vec![], None),
            CellData::new(config.modpacks.enabled.join(","), Some(Color::Magenta), vec![], None))
        ], Some(UTF8_HORIZONTAL_ONLY));
        
        notice("Run this command again with --force to enable it anyway..", Some(Color::Yellow), vec![]);
        return Err(RustiqueError::SimpleError(format!("Modpacks already enabled {}", config.modpacks.enabled.join(","))));
        
    }
    
    // we know that the modpack is installed and IS NOT enabled
    // lets enable it
    
    // get list of mods for the modpack
    // create symlinks is the Mods folder
    // return the modpack id that was enabled
    
    let mod_list: Vec<FileName> = extract_all_mods_metadata(&mod_pack_install_dir, false).await?
        .keys()
        .cloned()
        .collect();
    
    for m in mod_list {
        let target = &mod_pack_install_dir.join(&m);
        let link = mod_dir.as_ref().join(&m);
        SymlinkManager::create(target, link).await?;
    }
    
    
    Ok(args.mpk_id)
}