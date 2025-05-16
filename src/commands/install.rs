use crate::aliases::ModID;
use crate::api::client::ApiClient;
use crate::commands::sync::{get_sync_data};
use crate::install_manager::{install_manager, Install};
use crate::rustique_errors::RustiqueError;
use crate::rustique_errors::RustiqueError::SimpleError;
use crate::utils::{extract_all_mods_metadata, gather_missing_dependencies};
use crate::version_management::{parse_latest_version};
use std::path::PathBuf;
use tracing::{debug, info};
use crate::information_utils::{display_installation_results, notice};

// Report if trying install a mod that already exists
// Use -f to force an installation
// add way to set the version you want to download
pub async fn install_cmd(mod_dir: &PathBuf, mods_requested: Vec<ModID>, force: bool) -> Result<(), RustiqueError> {

    // get sync data
    let sync_data = get_sync_data(mod_dir).await?;

    let installed_mods = sync_data.rustique_sync.clone();
    // remove any mods from mods_requested if the exist in installed_mods

    let mods_requested_cleaned : Vec<ModID>  = mods_requested.iter().filter(|&id| !installed_mods.contains_key(id) && !force).cloned().collect();

    if mods_requested.is_empty() {
        notice("Looks like you have all the mods requested. If you would like to reinstall them, run this command again with --force", Some(comfy_table::Color::Yellow), vec![]);
        return Err(SimpleError("No mods to install".to_string()))
    }

    let client = ApiClient::new();

    // get the download urls for all requested mods
    let result = client.fetch_mods_parallel(mods_requested_cleaned).await?;

    let mods_requested: Vec<Install> =
        result.into_iter().map(|(mod_id, mod_info)| {
            let (version, download_url, _) = parse_latest_version(&mod_info.mod_json.releases);
            Install {
                mod_id: mod_id.clone(),
                mod_name: mod_info.mod_json.name.unwrap_or_default(),
                version_to_install: version,
                download_url: download_url.clone(),
                current_file_path: None,
            }
        }).collect();


    info!("Mods requested {:?}", mods_requested);

    let mods_processed = install_manager(mod_dir, mods_requested.clone(), installed_mods).await?;

    display_installation_results(mods_processed);

    Ok(())
}


pub async fn install_missing_deps(mod_dir: &PathBuf, mods_requested: Vec<ModID>) -> Result<(), RustiqueError> {

    // get all installed mod info
    // retrieve all dependencies
    // send missing ones to install_manager()

    let installed_mods = extract_all_mods_metadata(mod_dir).await?;
    let sync_data = get_sync_data(mod_dir).await?.rustique_sync.clone();


    // if there are reports of slowness is this section .values().par_bridge()...flat_map_iter() could be used to speed it up
    // this is prob not an issue even with a lot of mods as the data is all in memory at this point
    let mut missing_deps: Vec<Install> = gather_missing_dependencies(&installed_mods, &mods_requested, &sync_data);

    let client = ApiClient::new();

    // get the final list of mods we know need to be installed
    let md_ids: Vec<ModID> = missing_deps.iter().map(|i| i.mod_id.clone()).collect();

    // get download_urls
    let result = client.fetch_mods_parallel(md_ids).await?;

    for mod_info in &mut missing_deps {
        if let Some(data) = result.get(&mod_info.mod_id) {
            mod_info.mod_name = data.mod_json.name.clone().unwrap_or_default();
            let (version, download_url, _) = parse_latest_version(&data.mod_json.releases);
            mod_info.download_url = download_url;
            mod_info.version_to_install = version;
        }
    }

    debug!("deps: {:?}", missing_deps);

    let mods_processed = install_manager(mod_dir, missing_deps, sync_data).await?;


    info!("mods_processed {:#?}", mods_processed);

    display_installation_results(mods_processed);

    Ok(())
}


