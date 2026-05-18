use std::collections::HashMap;
use comfy_table::{Attribute, CellAlignment, Color};
use comfy_table::presets::UTF8_HORIZONTAL_ONLY;
use rustique_core::aliases::{ModID, ModVersion};
use rustique_core::api::client::ApiClient;
use crate::commands::sync::{get_sync_data};
use rustique_core::install_manager::{install_manager, Install};
use rustique_core::rustique_errors::RustiqueError;
use rustique_core::rustique_errors::RustiqueError::SimpleError;
use rustique_core::utils::{extract_all_mods_metadata, gather_missing_dependencies, split_modid_version};
use rustique_core::version_management::{parse_latest_version, parse_pinned_version};
use tracing::{debug, info};
use rustique_core::config::config_manager::{get_config, Package};
use rustique_core::information_utils::{command_output, display_incompatible_mods_constraint, display_installation_results, display_table, notice, rustique_message, CellData, RustiqueMessage};
use rustique_core::traits::ref_ext::PathRef;

// Report if trying install a mod that already exists
// Use -f to force an installation
pub async fn install_cmd(mod_dir: impl PathRef, mods_requested: Vec<ModID>, _force: bool) -> Result<(), RustiqueError> {
    let mod_dir = mod_dir.as_ref();
    info!("install_cmd: {mods_requested:?}");
    
    display_table(vec![command_output("Installing..", mods_requested.join(", "))], Some(UTF8_HORIZONTAL_ONLY));
    
    // do this first as we need to strip the @ if it exists
    let mod_map: HashMap<ModID, Option<ModVersion>> = mods_requested.iter().map(split_modid_version).collect();
    let mut no_compatible_mods: Vec<String> = Vec::new();
    
    
    // get sync data
    let sync_data = get_sync_data(mod_dir, true).await?;
    
    let config = get_config().read().await;
    
    let installed_mods = sync_data.rustique_sync.clone();
    
    let client = ApiClient::new();

    // get the download urls for all requested mods
    let result = client.fetch_mods_parallel(mod_map.keys().cloned().collect()).await?;
    
    if result.is_empty() {
        return Err(SimpleError(format!("Invalid modid {mods_requested:?}")));
    }

    let mods_requested: Vec<Install> =
        result.into_iter().filter_map(|(mod_id, mod_info)| {
            let mod_id = mod_id.to_lowercase();
            // println!("Trying to install {mod_id}");
            let pinned_version = if let Some(mod_version) =  mod_map.get(&mod_id) {
                if mod_version.is_some() {
                    // println!("Mod {mod_id} version found {}", mod_version.clone().unwrap());
                    
                    Some(mod_version.clone().unwrap())
                } else if let Some(package) = config.pkg.iter().find(|package| package.mod_id == mod_id) {
                    // println!("Mod {mod_id} package found {package:?}");
                    package.pinned_version.clone()
                } else {
                    None
                }
            } else {
                None
            };
            
            
            let pkg = Package {
                mod_id: mod_id.clone(),
                pinned_version,
            };
            
            info!("pkg: {:?}", pkg);
            
            let pinned_game_ver = &config.pinned_game_version;
            
            let (version, download_url, _,_) = match parse_pinned_version(&mod_info.mod_json.releases, &pkg, pinned_game_ver, config.allow_unstable) {
                Ok(pv) => pv,
                Err(e) => {
                    let mod_str = format!("{}\t- {}", &mod_info.mod_json.mod_id, &mod_info.mod_json.name.clone().unwrap_or(String::new()));
                    no_compatible_mods.push(String::from(mod_str.trim()));
                    info!("Incompatible mod constraints for {mod_str} {e}");

                    return None
                },
            };

            let x =  Install {
                mod_id: mod_id.clone(),
                mod_name: mod_info.mod_json.name.unwrap_or_default(),
                version_to_install: version,
                download_url: download_url.clone(),
                current_file_path: None,
            };

            info!("Installing.. {x:?}");

            Some(x)

        }).collect();


    // Collect mods that have no compatible version into a vec and display them in a nice table to avoid message spam

    if !no_compatible_mods.is_empty() {
        display_incompatible_mods_constraint(no_compatible_mods, "Failed to install mods due to pinned constraints".into());
    }

    info!("Mods requested {:?}", mods_requested);

    let mods_processed = install_manager(mod_dir, mods_requested.clone(), installed_mods).await?;

    display_installation_results(mods_processed);

    Ok(())
}


/// mod_dir_for_req is where the mods_requested will be searched for
/// all dependencies will be installed to dep_install_path
pub async fn install_missing_deps<V: AsRef<[ModID]>>(mod_dir_for_req: impl PathRef, mods_requested: V, dep_install_path: impl PathRef) -> Result<(), RustiqueError> {
    let (mod_dir , mods_requested) = (mod_dir_for_req.as_ref(), mods_requested.as_ref());
    // get all installed mod info
    // retrieve all dependencies
    // send missing ones to install_manager()

    let installed_mods = extract_all_mods_metadata(mod_dir, true).await?;
    // silence the sync message because it happens too much during installation.
    let sync_data = get_sync_data(mod_dir, true).await?.rustique_sync.clone();

    let mods_map: HashMap<ModID, Option<ModVersion>> = mods_requested.iter().map(split_modid_version).collect();
    let mods_id_vec: Vec<ModID> = mods_map.keys().cloned().collect();
    
    info!("install_missing_deps: mods_id_vec: {:?}", mods_id_vec);

    // if there are reports of slowness is this section .values().par_bridge()...flat_map_iter() could be used to speed it up
    // this is prob not an issue even with a lot of mods as the data is all in memory at this point
    let mut missing_deps: Vec<Install> = gather_missing_dependencies(&installed_mods, &mods_id_vec, &sync_data);

    let client = ApiClient::new();

    // get the final list of mods we know need to be installed
    let md_ids: Vec<ModID> = missing_deps.iter().map(|i| i.mod_id.clone()).collect();
    info!("md_ids: {:?}", md_ids);
    // get download_urls
    let result = client.fetch_mods_parallel(md_ids.clone()).await?;
    info!("result: {:?}", result);
    
    if result.is_empty() {
        info!("No missing deps to download..");
        return Ok(())
    }

    for mod_info in &mut missing_deps {
        if let Some(data) = result.get(&mod_info.mod_id) {
            mod_info.mod_name = data.mod_json.name.clone().unwrap_or_default();
            let (version, download_url, _,_) = parse_latest_version(&data.mod_json.releases);
            mod_info.download_url = download_url;
            mod_info.version_to_install = version;
        }
    }

    debug!("deps: {:?}", missing_deps);

    let mods_processed = install_manager(dep_install_path, missing_deps, sync_data).await?;


    info!("mods_processed {:#?}", mods_processed);

    display_installation_results(mods_processed);

    Ok(())
}


