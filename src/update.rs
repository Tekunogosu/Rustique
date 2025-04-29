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
use crate::install::{install_mod};
use crate::rustique_errors::RustiqueError;

pub fn update_mods(mod_dir: &PathBuf, update_mod_ids: Vec<String>, keep_old_files: bool) -> Result<(), RustiqueError> {
    eprintln!("{}", "Updating mods...".green().bold());

    let sync_data  = parse_sync_file(mod_dir);
    if sync_data.is_ok() {
        let sync_data = sync_data?;

        let mut mods_to_update: Vec<ModSyncInfo> = Vec::new();
        let mut updates_exist = false;

        if !update_mod_ids.is_empty() {
            update_mod_ids.iter().for_each(|typed_mod_id| {
                let mod_sync_data = &sync_data.rustique_sync;
                // user typed in a valid typed_mod_id so violet is happy now
                let typed_mod_id = typed_mod_id.to_lowercase();
                if mod_sync_data.contains_key(&typed_mod_id) {
                    mods_to_update.push(mod_sync_data[&typed_mod_id].clone());
                    updates_exist = true;
                } else {
                    eprintln!("{} is not a valid mod_id!", &typed_mod_id.red());
                }
            });
        } else {
            mods_to_update = sync_data.rustique_sync.values().cloned().collect();
            updates_exist = true;
        }

        if !updates_exist {
            return Err(RustiqueError::SimpleError(String::from("No valid update ids..\n\r")))
        }

        mods_to_update.par_iter().for_each(|mod_sync_info| {
            match (|| {
                if mod_sync_info.latest_known_version != mod_sync_info.installed_version {
                    let old_filename = if keep_old_files == false {
                        Some(mod_sync_info.file_name.to_string())
                    } else {
                        None
                    };

                    update_mod(mod_dir, mod_sync_info.latest_download_url.clone(), old_filename)

                } else {
                    Ok(())
                }
            })() {
                Ok(_res) => {}
                Err(e) => println!("{}", e.to_string()),
            }
        });

    } else {
        println!("Looks like you need to run './Rustique sync' first");
        exit(1);
    }

    Ok(())
}

pub fn update_mod(mod_dir: &PathBuf, latest_download_url: String, old_filename: Option<String>) -> Result<(), RustiqueError> {

    // download_mod(mod_dir, latest_download_url)?;

    install_mod(mod_dir, ModDownload::DownloadURL(latest_download_url), None)?;

    if let Some(old_filename) = old_filename {
        let old_filepath = &mod_dir.clone().join(old_filename.to_string());
        delete_file(old_filepath)?
    }

    Ok(())
}
