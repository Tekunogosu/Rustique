use std::collections::HashMap;
use std::path::{Path, PathBuf};
use comfy_table::{CellAlignment, Color};
use comfy_table::presets::UTF8_BORDERS_ONLY;
use tokio::fs::ReadDir;
use tracing::{info, warn};
use core::aliases::{ModFileName, ModID, ModVersion};
use crate::commands::arg_structs::delete_args::DeleteArgAllVals;
use crate::commands::sync::get_sync_data;
use core::config::config_manager::get_config;
use core::information_utils::{display_table, CellData};
use core::symlink_manager::SymlinkManager;
use core::rustique_errors::RustiqueError;
use core::traits::ref_ext::PathRef;
use core::utils::{delete_file, extract_all_mods_metadata, split_modid_version, iterate_and_delete};
use core::version_management::parse_version;

pub async fn delete_all(mod_dir: impl PathRef, delete_type: &DeleteArgAllVals) -> Result<(), RustiqueError> {
    
    let config = get_config().read().await;
    
   
    // location_type: Mods looks at the folder specified by mod_dir
    // location_type: Backups looks at the backup dir in the config
    // location_type: Both does both

    let mut cleaned_mods: Vec<PathBuf> = Vec::new();
    
    if matches!(delete_type, DeleteArgAllVals::Mods) || matches!(delete_type, DeleteArgAllVals::Both) {
        // delete all mods in the mod_dir
        // collect paths for all in mod_dir
        // use delete_file on each 
        
        let mut mods = tokio::fs::read_dir(mod_dir).await?;
        iterate_and_delete(&mut mods, &mut cleaned_mods).await?;
    }
    
    if matches!(delete_type, DeleteArgAllVals::Backups) || matches!(delete_type, DeleteArgAllVals::Both) {
       
        let mut mods = tokio::fs::read_dir(Path::new(&config.backup_mods_dir)).await?;
        iterate_and_delete(&mut mods, &mut cleaned_mods).await?;
    }
    
    show_deleted(&format!("{cleaned_mods:?}"));
    
    Ok(())
}

pub async fn delete_cmd(mod_dir: impl PathRef, mod_ids: Vec<ModID>, is_backup: bool) -> Result<(), RustiqueError> {
    
    let config = get_config().read().await;
   
    let mut mod_lookup: HashMap<ModID, Option<ModVersion>> = mod_ids.iter().map(split_modid_version).collect();
   
    info!("mod_lookup {:?}", mod_lookup);
    
    let mod_dir = if is_backup {
        Path::new(&config.backup_mods_dir)
    } else {
        mod_dir.as_ref()
    };
    
    // grab only the real mods in the m_dir, ignoring the symlinks (modpacks)
    let all_metadata  = extract_all_mods_metadata(mod_dir, true).await?;
    let mut processed_mods: Vec<(ModID, ModFileName)> = Vec::new();
   
    for (filename, modinfo) in all_metadata {
        if let Some((_, target_version)) = mod_lookup.remove_entry(&modinfo.mod_id) {
            info!("target_version: {:?}", target_version);
            
            let should_delete = match &target_version {
                Some(required_version) => {
                    let required = parse_version(required_version)?;
                    let current = parse_version(&modinfo.version.clone().unwrap_or("0.0.0".into()))?;
                    info!("DELETE: Comparing versions: {} == {}", required, current);
                    required == current
                }
                None => true,
            };
            
            if should_delete {
                processed_mods.push((format!("{}@{}", modinfo.mod_id, modinfo.version.unwrap_or(String::new())), filename.clone()));
                delete_file(mod_dir.join(&filename)).await?;
            } else {
                warn!("{:?}@{:?} not found", modinfo.mod_id, target_version);
            }
        } 
    }
    
    if !processed_mods.is_empty() {
        // get sync data and remove all the processed_mods from it. (this saves having to sync again)
        let mut sync_data = get_sync_data(&mod_dir, true).await?;

        for pm in &processed_mods {
            let (mod_id, version) = split_modid_version(&pm.0);
            
            if let Some(rem_data) = sync_data.rustique_sync.remove(&mod_id) {
                if rem_data.installed_version != version.unwrap_or(String::new()) {
                    // value didn't match, put the entry back into the sync file. 
                    // Sync file will only have the latest version, so this prob means it was on old version that was removed.
                    sync_data.rustique_sync.insert(mod_id, rem_data);
                }
            }
            
        }

        // save the file
        sync_data.save().await?;
    }
    
    let removed = processed_mods.iter().map(|m|format!("{}:{}",m.0, m.1)).collect::<Vec<String>>().join("], [");

    show_deleted(&removed);

    if !mod_lookup.is_empty() {
        info!("Unable to find {:?}", mod_lookup);
    }
    
    Ok(())
}

fn show_deleted(deleted_mods: &str) {
    display_table(
        vec![
            (
                CellData::new("Successfully deleted:".into(), Some(Color::Green), vec![], None),
                CellData::new(format!("[{deleted_mods}]"), Some(Color::Magenta), vec![], Some(CellAlignment::Right))
            )
        ],
        Some(UTF8_BORDERS_ONLY)
    );
}