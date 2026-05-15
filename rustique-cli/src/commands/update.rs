
use crate::commands::sync::{get_sync_data};
use owo_colors::OwoColorize;
use comfy_table::{Attribute, Color};
use std::collections::{BTreeMap, HashMap};
use std::path::{PathBuf};
use std::time::Instant;
use tracing::debug;
use tracing::span::Attributes;
use rustique_core::config::config_manager::get_config;
use rustique_core::information_utils::{display_installation_results, elapsed_footer, notice};
use rustique_core::sync_structs::ModSyncInfo;
use rustique_core::traits::ref_ext::PathRef;
use rustique_core::install_manager::{install_manager, Install, Installed};
use rustique_core::rustique_errors::RustiqueError;
use rustique_core::utils::{backup_older_files, remove_older_files, split_modid_version};
use rustique_core::aliases::ModID;

#[allow(clippy::map_entry)]
pub async fn update_mods<V: AsRef<[ModID]>>(mod_dir: impl PathRef, update_mod_ids: V, keep_old_files: bool) -> Result<(), RustiqueError> {
    let (mod_dir, update_mod_ids) = (mod_dir.as_ref(), update_mod_ids.as_ref());
    let start_time = Instant::now();
    let config = get_config().read().await;
    let sync_data = get_sync_data(&PathBuf::from(mod_dir), false).await?;
    
    notice("Updating mods...", Option::from(Color::Yellow), vec![Attribute::Bold]);
    // filter out anything that is a symlink. This means it's a modpack file we don't want to update. 
    let sync_data = sync_data.rustique_sync
        .into_iter()// Consume and transform
        .filter_map(|(mod_id,sync_info)| {
            // filter out any symlinks and return a modid that has the version stripped from it
            if sync_info.is_symlink {
                None
            } else {
                Some((split_modid_version(mod_id).0, sync_info))
            }

        })
        .collect();
    
    let mut mods_to_check_update: BTreeMap<ModID, ModSyncInfo> = BTreeMap::new();
    let mut updates_exist = false;

    if update_mod_ids.is_empty() {
        mods_to_check_update.clone_from(&sync_data);
        updates_exist = true;
    } else {
        for typed_mod_id in update_mod_ids {

            // user typed in a valid typed_mod_id so violet is happy now
            let typed_mod_id = typed_mod_id.to_lowercase();
            if sync_data.contains_key(&typed_mod_id) {
                mods_to_check_update.entry(typed_mod_id.clone()).or_insert(sync_data[&typed_mod_id].clone());
                updates_exist = true;
            } else {
                notice(format!("{} is not a installed mod, check your modid and try again", typed_mod_id.red()), Some(Color::Yellow), vec![Attribute::Bold]);
            }
        }
    }

    if !updates_exist {
        return Err(RustiqueError::SimpleError(String::from("No valid update ids or the mod dir is empty..\n\r")))
    }

    let all_installed_mods: BTreeMap<ModID, ModSyncInfo> = mods_to_check_update.clone();
    debug!("all_installed_mods: {:#?}", all_installed_mods);

    let final_mod_update_list: Vec<Install> = mods_to_check_update
        .into_iter()
        .filter_map(|(mod_id, mod_sync_info)| {
           
            // if mod_id is present in the [[pkg]] section of the config, check if we are allowed to update the mod
            if mod_sync_info.latest_known_version != mod_sync_info.installed_version 
                && !mod_id.is_empty() { 
                Some(Install { 
                    mod_id: mod_id.to_lowercase(),
                    mod_name: mod_sync_info.mod_name.clone(),
                    version_to_install: mod_sync_info.latest_known_version.clone(),
                    download_url: mod_sync_info.latest_download_url.clone(),
                    current_file_path: Some(mod_dir.join(mod_sync_info.file_name)),
                })
            } else {
                None
            }
    }).collect();

    debug!("final_mod_update_list: {:#?}", final_mod_update_list);


    let mods_processed: Vec<Installed> = install_manager(mod_dir, final_mod_update_list.clone(), all_installed_mods).await?;
    
    if config.backup_mods {
        backup_older_files(&mods_processed).await?;        
    }
    
    if !keep_old_files {
        remove_older_files(&mods_processed).await?;
    }
    
    display_installation_results(mods_processed);


    if config.show_execution_time {
        elapsed_footer(start_time, "Update");
    }

    Ok(())
}

