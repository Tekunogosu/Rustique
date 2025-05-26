
use std::path::Path;
use comfy_table::Color;
use tracing::{debug, error, info};
use owo_colors::OwoColorize;
use crate::api::api_structs::ModInfo;
use crate::api::client::ApiClient;
use crate::api::download::download_requested_mods;
use crate::commands::arg_structs::modpack_args::MPUpdateArgs;
use crate::commands::sync::{sync, RustiqueSyncJson};
use crate::commands::update::update_mods;
use crate::config::config_manager::{get_config, Package};
use crate::consts::{FILE_MODINFO_JSON, FILE_RUSTIQUE_SYNC};
use crate::information_utils::notice;
use crate::install_manager::{Install};
use crate::modpack::mp_install::check_if_mp_enabled;
use crate::rustique_errors::RustiqueError;
use crate::utils::{delete_file, extract_zip_metadata, parse_json_file};

pub async fn mp_update(args: MPUpdateArgs) -> Result<(), RustiqueError> {

    let config = get_config().read().await;
   
    // Make sure the modpack isn't enabled or we'll have orphaned symlinks
    check_if_mp_enabled(&args.mpk_id, &config.modpacks.enabled);
    
    
    let modpack_base_dir = Path::new(&config.modpacks.modpack_dir);
    let pack_dir = modpack_base_dir.join("packs");
    // first check if the sync file in the modpack dir exists, if not, run sync on this location
    let modpack_sync_file = match parse_json_file::<RustiqueSyncJson>(&pack_dir.join(FILE_RUSTIQUE_SYNC)) {
        Ok(sync_data) => sync_data,
        Err(e) => {
            info!("Failed getting sync file for packs, {}", e.to_string().red());
            sync(&pack_dir,false, vec![]).await?;
            parse_json_file::<RustiqueSyncJson>(&pack_dir)?
        }
    };

    // check if the requested modpack is in the sync file
    if !modpack_sync_file.rustique_sync.contains_key(&args.mpk_id) {
        notice(format!("{} doesn't appear to be installed. Use Rustique modpack install {} to download the modpack", &args.mpk_id, &args.mpk_id), Some(Color::Yellow), vec![]);
        return Err(RustiqueError::SimpleError("Modpack not installed, nothing to update".into()));
    }
    let Some(modpack_info) = modpack_sync_file.rustique_sync.get(&args.mpk_id) else {
        return Err(RustiqueError::SimpleError("Unable to retrieve modpack info from sync file".into()));
    };


    // get the modinfo.json file for the modpack and check it against the sync file
    if modpack_info.installed_version.eq_ignore_ascii_case(&modpack_info.latest_known_version) {
        notice(format!("Modpack {} is already up to date!", &args.mpk_id), Some(Color::Green), vec![]);
        return Ok(());
    }

    // we know its not up-to-date, download the latest version and save it to the packs folder, deleting the old version.. unless the are named the same
    let mp_file_path = pack_dir.join(&modpack_info.file_name);
    let client = ApiClient::new();
    // we already have the latest download URL, use that
    let m_install = Install {
        mod_id: args.mpk_id.clone(),
        mod_name: modpack_info.mod_name.clone(),
        version_to_install: modpack_info.latest_known_version.clone(),
        download_url: modpack_info.latest_download_url.clone(),
        current_file_path: Some(mp_file_path.clone()),
    };
    
    debug!("{} {:#?}","m_install".green(), m_install.blue());

    let installed = match download_requested_mods(&pack_dir, &mut vec![m_install], &client).await {
        Ok(i) => {
            // delete the old file if its named differently from the new
            // there is only 1 file as we only process 1 modpack at a time
            if i.first().is_some_and(|e| !e.installed_file_path.eq(&Some(mp_file_path.clone()))) {
                info!("Deleting old modpack file {}", mp_file_path.display());
                delete_file(&mp_file_path).await?;
            }

            i.first().unwrap().clone()
        },
        Err(e) => return Err(e)
    };
    
    let Some(updated_mp_filepath) = &installed.installed_file_path else {
        return Err(RustiqueError::SimpleError(format!("Unable to get updated file path for {}", &args.mpk_id)));
    };
    
    let mp_mod_pkgs: Vec<Package> = extract_zip_metadata::<ModInfo>(&updated_mp_filepath, FILE_MODINFO_JSON)?.dependencies.iter().map(|(mod_id, mod_version)| Package {
        mod_id: mod_id.clone(),
        pinned_version: Some(mod_version.clone()),
    }).collect();
    
    let mp_install_dir = modpack_base_dir.join("installed").join(&args.mpk_id);
    sync(&mp_install_dir, true, &mp_mod_pkgs).await?;
    
    match update_mods(&mp_install_dir, &[], false).await {
        Ok(()) => {
            sync(&mp_install_dir, false, &mp_mod_pkgs).await?;
        },
        Err(e) => {
            error!("{}", e.to_string());
        }
    }
    
    Ok(())
}
