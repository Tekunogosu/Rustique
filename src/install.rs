use crate::aliases::{ModID, ModVersion};
use crate::api::ApiClient;
use crate::rustique_errors::RustiqueError;
use crate::utils::{
    ModDownload, dlog, download_mod, extract_all_mods_metadata, extract_valid_dependencies,
    extract_zip_metadata, get_installed_mods,
};
use colored::Colorize;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub fn install_mod(
    mod_dir: &PathBuf,
    mod_to_download: ModDownload,
    api: Option<ApiClient>,
) -> Result<(), RustiqueError> {
    // get mod_id from api so we have the latest download_url
    let api = api.unwrap_or_else(ApiClient::new);

    println!("ModDownload: {:?}", mod_to_download);

    let download_url = match mod_to_download.clone() {
        ModDownload::ModID(mod_id) => {
            let mod_info = api
                .fetch_mod(&mod_id)
                .map_err(|e| RustiqueError::ApiError {
                    context: format!("Failed to fetch mod_id: {}", mod_id),
                    source: e,
                })?;
            &mod_info.mod_json.releases[0].main_file.clone().unwrap()
        }
        ModDownload::DownloadURL(download_url) => &download_url.clone(),
    };

    // we have the download_url, download the mod into the mods dir
    dlog(&format!("Downloading mod_file: {}", download_url));
    match download_mod(mod_dir, &download_url) {
        Ok(mod_info) => eprintln!("{} successfully installed", mod_info.mod_id.green()),
        Err(e) => eprintln!("Failed to download mod: {}", e.to_string()),
    }

    // install_mods handles the multithreading and dependencies when used
    // but update has its own multithreading, so we only call this here if we get 1 download URL
    // which is assumed to be update
    // TODO:: This is pretty ugly tbh and could use a good refactor..
    if matches!(mod_to_download, ModDownload::DownloadURL(_)) {
        install_missing_dependencies(mod_dir)?
    }

    Ok(())
}

pub fn install_mods(
    mod_dir: &PathBuf,
    mod_ids: Vec<String>,
) -> Result<Vec<Result<(), RustiqueError>>, RustiqueError> {
    let api = ApiClient::new();

    // we use .map here as a simple way to capture any RustiqueError results and pass them up to the main
    // match for displaying of error message

    let result: Vec<Result<(), RustiqueError>> = mod_ids
        .par_iter()
        .map(|mod_id| {
            install_mod(
                mod_dir,
                ModDownload::ModID(mod_id.to_string()),
                Some(api.clone()),
            )
        })
        .collect();

    install_missing_dependencies(&mod_dir)?;

    Ok(result)
}

pub fn install_missing_dependencies(mod_dir: &PathBuf) -> Result<(), RustiqueError> {
    println!("Installing missing dependencies...");
    let metadata = extract_all_mods_metadata(mod_dir)?;
    let all_installed_mods: Vec<ModID> = metadata
        .values()
        .map(|mod_info| mod_info.mod_id.clone())
        .collect();

    let missing_dependencies: Arc<Mutex<HashSet<ModID>>> = Arc::new(Mutex::new(HashSet::new()));

    metadata.par_iter().for_each(|(mod_id, mod_info)| {
        let missing =
            extract_valid_dependencies(mod_info.dependencies.clone(), &all_installed_mods);
        println!("{}: {}", mod_id, missing.join("\n"));
        missing_dependencies
            .lock()
            .unwrap()
            .extend(missing.into_iter());
    });

    let final_list: Vec<ModID> = Arc::try_unwrap(missing_dependencies)
        .map_err(|_| RustiqueError::SimpleError("Failed to unwrap Arc".to_string()))?
            .into_inner()
        .map_err(|_| RustiqueError::SimpleError("Failed to unlock mutex".to_string()))?
            .into_iter()
        .collect();

    if !final_list.is_empty() {
        install_mods(mod_dir, final_list)?;
    }

    Ok(())
}
