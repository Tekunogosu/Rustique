use std::collections::HashMap;
use std::fmt::format;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use crate::api::ApiClient;
use crate::sync::{parse_sync_file, ModSyncInfo};
use crate::utils::{delete_file, dlog, RustiqueOptions, download_mod, ModDownload};
use rayon::prelude::*;
use std::process::exit;
use colored::Colorize;
use url::{form_urlencoded, Url};
use crate::aliases::ModID;
use crate::install::{install_mod, install_mods, InstallOrUpdate};
use crate::rustique_errors::RustiqueError;

pub fn update_mods(mod_dir: &PathBuf, update_mod_ids: Vec<ModID>, _keep_old_files: bool) -> Result<(), RustiqueError> {
    eprintln!("{}", "Updating mods...".green().bold());

    let sync_data  = parse_sync_file(mod_dir);
    if sync_data.is_ok() {
        let sync_data = sync_data?;

        let mut mods_to_check_update: HashMap<ModID, ModSyncInfo> = HashMap::new();
        let mut updates_exist = false;

        if !update_mod_ids.is_empty() {
            update_mod_ids.iter().for_each(|typed_mod_id| {
                let mod_sync_data = &sync_data.rustique_sync;
                // user typed in a valid typed_mod_id so violet is happy now
                let typed_mod_id = typed_mod_id.to_lowercase();
                if mod_sync_data.contains_key(&typed_mod_id) {
                    mods_to_check_update.entry(typed_mod_id.clone()).or_insert(mod_sync_data[&typed_mod_id].clone());
                    updates_exist = true;
                } else {
                    eprintln!("{} is not a valid mod_id!", &typed_mod_id.red());
                }
            });
        } else {
            mods_to_check_update = sync_data.rustique_sync.clone();
            updates_exist = true;
        }

        if !updates_exist {
            return Err(RustiqueError::SimpleError(String::from("No valid update ids..\n\r")))
        }

        let final_mod_update_list: HashMap<ModID, ModSyncInfo> = mods_to_check_update
            .into_iter().filter(|(_, mod_sync_info)| {
                mod_sync_info.latest_known_version != mod_sync_info.installed_version
            }).collect();


        install_mods(mod_dir, InstallOrUpdate::Update(final_mod_update_list.clone()))?;

        if !_keep_old_files {
            final_mod_update_list.iter().for_each(|(_, mod_sync_info)| {
                let file_path = &mod_dir.clone().join(mod_sync_info.file_name.to_string());
                match delete_file(file_path) {
                    Ok(_) => (),
                    Err(e) => {
                        println!("Error deleting file {}: {}", file_path.display(), e);
                    }
                }
            })
        }

    } else {
        println!("Looks like you need to run './Rustique sync' first");
        exit(1);
    }

    Ok(())
}
