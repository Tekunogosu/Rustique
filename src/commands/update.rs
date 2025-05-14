use crate::aliases::ModID;
use crate::commands::sync::{parse_json_file, ModSyncInfo, RustiqueSyncJson, SYNC_FILE_NAME};
use crate::config_manager::get_config;
use crate::install_manager::{install_manager, Install, Installed};
use crate::rustique_errors::RustiqueError;
use crate::utils::{delete_file, display_installation_results, elapsed_footer, notice};
use owo_colors::OwoColorize;
use comfy_table::{Attribute, Color};
use std::collections::HashMap;
use std::path::{PathBuf};
use std::process::exit;
use std::time::Instant;
use tracing::{debug, info};


#[allow(clippy::map_entry)]
pub async fn update_mods(mod_dir: &PathBuf, update_mod_ids: Vec<ModID>, keep_old_files: bool) -> Result<(), RustiqueError> {
    let start_time = Instant::now();
    let config = get_config().read().await;
    let sync_data = parse_json_file::<RustiqueSyncJson>(&PathBuf::from(mod_dir).join(SYNC_FILE_NAME));

    if sync_data.is_ok() {
        notice("Updating mods...", Option::from(Color::Yellow), vec![Attribute::Bold]);
        let sync_data = sync_data?;
        let mut mods_to_check_update: HashMap<ModID, ModSyncInfo> = HashMap::new();
        let mut updates_exist = false;

        if update_mod_ids.is_empty() {
            mods_to_check_update.clone_from(&sync_data.rustique_sync);
            updates_exist = true;
        } else {
            for typed_mod_id in &update_mod_ids {
                let mod_sync_data = &sync_data.rustique_sync;
                // user typed in a valid typed_mod_id so violet is happy now
                let typed_mod_id = typed_mod_id.to_lowercase();
                if mod_sync_data.contains_key(&typed_mod_id) {
                    mods_to_check_update.entry(typed_mod_id.clone()).or_insert(mod_sync_data[&typed_mod_id].clone());
                    updates_exist = true;
                } else {
                    println!("{} is not a valid mod_id!", &typed_mod_id.red());
                }
            }
        }

        if !updates_exist {
            return Err(RustiqueError::SimpleError(String::from("No valid update ids or the mod dir is empty..\n\r")))
        }

        let all_installed_mods: HashMap<ModID, ModSyncInfo> = mods_to_check_update.clone();
        debug!("all_installed_mods: {:#?}", all_installed_mods);

        let final_mod_update_list: Vec<Install> = mods_to_check_update
            .into_iter()
            .filter_map(|(mod_id, mod_sync_info)| {

            if mod_sync_info.latest_known_version != mod_sync_info.installed_version && !mod_id.is_empty() {
                Some(Install {
                    mod_id: mod_id.to_lowercase(),
                    mod_name: mod_sync_info.mod_name.clone(),
                    version_to_install: mod_sync_info.latest_known_version.clone(),
                    download_url: mod_sync_info.latest_download_url.clone(),
                    current_file_path: Some(mod_dir.clone().join(mod_sync_info.file_name)),
                })
            } else {
                None
            }
        }).collect();

        debug!("final_mod_update_list: {:#?}", final_mod_update_list);


        let mods_processed: Vec<Installed> = install_manager(mod_dir, final_mod_update_list.clone(), all_installed_mods).await?;

        if !keep_old_files {
            for mod_processed in &mods_processed {
                if let (Some(old), Some(new) )= (&mod_processed.old_file_path, &mod_processed.installed_file_path) {
                    if old == new {
                        info!("Old file and new file have the same name, **NOT DELETING**");
                    } else {
                        info!("Cleaning up mod file for {}", old.display());
                        delete_file(old).await?;
                    }
                }
            }
        }

        // display our results
        display_installation_results(mods_processed);

    } else {
        println!("{} {} {}", "Looks like you need to run".bright_yellow(), "'Rustique sync'".bright_blue().bold(), "first".yellow());
        exit(1);
    }

    if config.show_execution_time {
        elapsed_footer(start_time, "Update");
    }

    Ok(())
}

