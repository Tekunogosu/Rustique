use crate::aliases::{ModFileName, ModID, ModVersion};
use crate::api::ApiClient;
use crate::rustique_errors::RustiqueError;
use crate::utils::{
    dlog, download_mod, extract_all_mods_metadata, find_missing_dependencies,
    extract_zip_metadata,
};
use colored::Colorize;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};
use crate::api_structs::ModInfo;
use crate::sync::ModSyncInfo;
use crate::version_management::parse_latest_version;

pub enum InstallOrUpdate {
    Install(HashSet<ModID>),
    Update(HashMap<ModID, ModSyncInfo>),
}

#[derive(Clone, Debug)]
pub enum ModDownloadURI {
    ModID(String),
    DownloadURL(String),
}

impl ModDownloadURI {
    pub fn get_download_url(self, api: &ApiClient) -> Result<String, RustiqueError> {

        match self {
            ModDownloadURI::ModID(mod_id) => {
                let mod_info = api
                    .fetch_mod(&mod_id)
                    .map_err(|e| RustiqueError::ApiError {
                        context: format!("Failed to fetch mod_id: {}", mod_id),
                        source: e,
                    })?;

                let (_, download_url) = parse_latest_version(&mod_info.mod_json.releases);

                if download_url.is_empty() {
                    return Err(RustiqueError::SimpleError(format!("Download URL not found! {}", mod_id)));
                }

                Ok(download_url)
            }
            ModDownloadURI::DownloadURL(download_url) => Ok(download_url)
        }
    }
}

pub fn install_mod(
    mod_dir: &PathBuf,
    download_url: &String,
    api: &ApiClient,
) -> Result<(), RustiqueError> {
    // we have the download_url, download the mod into the mods dir
    dlog(&format!("Downloading mod_file: {}", download_url));
    match download_mod(mod_dir, &download_url, api) {
        Ok(mod_info) => eprintln!("{}: {} successfully installed", mod_info.mod_id.green(), mod_info.version.unwrap().yellow()),
        Err(e) => eprintln!("Failed to download mod: {}", e.to_string()),
    }

    Ok(())
}

pub fn install_mods(mod_dir: &PathBuf, install_or_update: InstallOrUpdate) -> Result<(), RustiqueError> {
    let api = ApiClient::new();

    // this vec is to tell the install_missing_dependencies which mods it update deps for
    let mut dep_filter_list: HashSet<ModID> = HashSet::new();
    // this is the actual update list of the mods that will be sent to install_mod(..)
    let mod_download_urls: Vec<String> = match install_or_update {
        InstallOrUpdate::Update(mod_ids) => {
            dep_filter_list = mod_ids.keys().cloned().collect();
            mod_ids
                .values()
                .cloned()
                .map(|mod_sync_info: ModSyncInfo| mod_sync_info.latest_download_url.clone())
                .collect()
        },
        InstallOrUpdate::Install(mod_ids) => {
            dep_filter_list.extend(mod_ids.clone());
            let mut urls = Vec::new();
            for mod_id in mod_ids {
                if let Ok(url) = ModDownloadURI::ModID(mod_id.clone()).get_download_url(&api) {
                    urls.push(url);
                }
            }
            urls
        }
    };

    if mod_download_urls.len() < 1 {
        Err(RustiqueError::SimpleError(format!("{}", "No valid mod ids found")))?
    }

    mod_download_urls.par_iter().for_each(|mod_download| {
       match install_mod(mod_dir, mod_download, &api) {
           Ok(_) => {}
           Err(e) => {
               eprintln!("{}", e);
           }
       }
    });

    install_missing_dependencies(&mod_dir, Option::from(dep_filter_list))?;

    Ok(())
}

pub fn install_missing_dependencies(mod_dir: &PathBuf, mods_to_update_deps: Option<HashSet<ModID>>) -> Result<(), RustiqueError> {
    eprintln!("{}","Checking for dependencies...".green().bold());

    let mut metadata: Vec<ModInfo> = extract_all_mods_metadata(mod_dir)?
        .into_values()
        .filter(|mod_info| {
            match mods_to_update_deps.as_ref() {
                Some(mods) => mods.contains(&mod_info.mod_id),
                None => true
            }
        })
        .collect();

    let mut seen_ids : HashSet<ModID> = HashSet::new();
    metadata.retain(|mod_info| {
       seen_ids.insert(mod_info.mod_id.clone())
    });

    let missing_dependencies: Arc<Mutex<HashSet<ModID>>> = Arc::new(Mutex::new(HashSet::new()));
    let mut exclude_updated_mods = mods_to_update_deps.unwrap_or_else(|| HashSet::new());

    // here we combine the seen_ids (which are all the unique mod_ids in our download dir
    // any anything passed to the function to exclude
    // TODO: this is really ugly, we are using 1 vec to make sure we update the mods from that vec
    // then later use it to exclude mods. it just feels weird..
    exclude_updated_mods.extend(seen_ids);

    metadata.par_iter().for_each(|mod_info| {
        let missing = find_missing_dependencies(mod_info.dependencies.clone(), Option::from(&exclude_updated_mods));

        missing_dependencies
            .lock()
            .unwrap()
            .extend(missing.into_iter());
    });

    let final_list: HashSet<ModID> = Arc::try_unwrap(missing_dependencies)
        .map_err(|_| RustiqueError::SimpleError("Failed to unwrap Arc".to_string()))?
            .into_inner()
        .map_err(|_| RustiqueError::SimpleError("Failed to unlock mutex".to_string()))?
            .into_iter()
        .collect();

    if !final_list.is_empty() {
        install_mods(mod_dir, InstallOrUpdate::Install(final_list))?;
    }

    Ok(())
}
