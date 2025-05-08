use crate::aliases::{DownloadURL, ModID, ModName, ModVersion};
use crate::api::api_structs::Mod;
use crate::api::client::{ApiClient};
use crate::api::download::download_requested_mods;
use crate::commands::sync::ModSyncInfo;
use crate::rustique_errors::RustiqueError;
use crate::utils::extract_zip_metadata;
use crate::version_management::parse_latest_version;
use rayon::prelude::*;
use std::collections::{HashMap};
use std::path::PathBuf;
use tracing::{error, info};

// install & update both will obtain the info needed to fill this struct
#[derive(Debug, Clone)]
pub struct Install {
    pub mod_id: ModID,
    pub mod_name: ModName,
    // Used with version pinning, otherwise ignored
    pub version_to_install: ModVersion,
    // download url of the version_to_install
    pub download_url: DownloadURL,
    // will be None if this is to be a fresh install
    pub current_file_path: Option<PathBuf>,
}


#[derive(Debug, Clone)]
pub struct Installed {
    pub mod_id: ModID,
    pub mod_name: ModName,
    pub installed_file_path: Option<PathBuf>,
    // will be None if this was a fresh install and not an update
    pub old_file_path: Option<PathBuf>,
    pub install_version: ModVersion,
    pub success: bool,
}



pub async fn install_manager(
    mod_dir: &PathBuf,
    mods_requested: Vec<Install>,
    installed_mods: HashMap<ModID, ModSyncInfo>) -> Result<Vec<Installed>, RustiqueError> {

    // this is the combined list of all mods installed, once download is completed, now mods will be
    // added here
    let mut total_mods_seen: HashMap<ModID, Installed> = HashMap::with_capacity(installed_mods.len());
    installed_mods.iter().for_each(|(mod_id, mod_sync_info)| {
        // this is what is already on the system
        // the version doesn't really matter, we just need to know modid and filepath, which the
        // info from sync would provide that
        total_mods_seen.insert(mod_id.clone(),Installed {
            mod_id: mod_id.clone(),
            mod_name: mod_sync_info.mod_name.clone(),
            installed_file_path: Some(mod_dir.join(mod_sync_info.file_name.clone())),
            success: true,
            old_file_path: Some(mod_dir.join(mod_sync_info.file_name.clone())),
            install_version: mod_sync_info.installed_version.clone(),
        });
    });


    // info!("total_mods_seen: {:#?}", total_mods_seen);
    // info!("mods_requested: {:#?}", mods_requested);

    let client = ApiClient::new();

    // This vec is filled and then consumed within download_requested_mods
    // each iteration of the loop will add new mods from dependencies to be processed next
    let mut mods_requested = mods_requested.clone();

    // Hold all the mods that were processed during this request.
    // We let the calling function handle what to do with failed installs
    let mut mods_processed: Vec<Installed> = Vec::new();

    let mut passes = 0;

    loop {
        // let mut recently_installed: Vec<Installed> = Vec::new();

        // this function will consume each value out of the mods_requested so we can rebuild it
        // after the dependencies check
       let recently_installed: Vec<Installed> =  match download_requested_mods(&mod_dir, &mut mods_requested, &client).await {
            Ok(processed_mods) => {
                info!("Successfully installed mods: {:?}", processed_mods);
                // update recently installed so we can get the dependencies
                mods_processed.extend(processed_mods.clone());
                processed_mods
            }
            Err(err) => {
                // TODO: This needs to be handled better I think..
                error!("Failed to install mods: {:?}", err);
                Vec::new()
            }
        };

        // add recently seen to total_mods_seen

        for installed in &recently_installed {
            total_mods_seen.insert(installed.mod_id.clone(), installed.clone());
        }


        // extract the modinfojson from recently_installed and gather the dependencies.
        // subtract any dependency which already resides in total seen mods

        let mut needed_dependencies: Vec<Install> = recently_installed.par_iter()
            .filter_map(|installed_mod| {
                let path = installed_mod.installed_file_path.clone()?;
                match extract_zip_metadata(path) {
                    Ok(mod_info) =>  {
                        Some(mod_info.dependencies
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|(dep_id, _)|{
                                    !dep_id.to_lowercase().contains("game")
                                    && !dep_id.to_lowercase().contains("creative")
                                    && !dep_id.to_lowercase().contains("survival")
                                    && !total_mods_seen.contains_key(dep_id.to_lowercase().as_str())
                            }).collect::<HashMap<_, _>>())
                    },
                    Err(err) => {
                        error!("Failed to extract zip metadata: {:?}", err);
                        None
                    }
                }
            })
            .flat_map_iter(|deps| deps.into_iter())
            .map(|(mod_id, mod_version)| Install {
                mod_id,
                mod_name: "".to_string(),
                version_to_install: mod_version,
                download_url: "".to_string(),
                current_file_path: None,
            }).collect();

        passes += 1;
        info!("pass: {}, needed_dependencies : {:?}", passes, needed_dependencies);

        if needed_dependencies.len() == 0 {
            break;
        }

        // obtain the download_urls for the currently needed dependencies and then pass it back to mods_requested
        let mod_ids: Vec<ModID> = needed_dependencies.iter().map(|dep| dep.mod_id.clone()).collect();
        let result: HashMap<ModID, Mod> = client.fetch_mods_parallel(mod_ids).await?;

        // info!("Mod api fetch result: {:#?}", result);

        // add the result to the mods_requested
        // obtain the latest download url
        // and the mod name from the HashMap and update the values in needed_deps
        // then dump needed_deps into requested_mods

        //TODO: double check needed values are present
        for mod_to_install in &mut needed_dependencies {
            if let Some(_mod) =  result.get(mod_to_install.mod_id.as_str()) {
                mod_to_install.mod_name = _mod.mod_json.name.clone().unwrap_or_default();
                // TODO: version pinning here??
                let (mod_version, download_url) = parse_latest_version(&_mod.mod_json.releases);
                mod_to_install.download_url = download_url;
                mod_to_install.version_to_install = mod_version;
            }
        }
        // seed the mods_requested and go again
        mods_requested.extend(needed_dependencies);
    }

    // TODO: Figure out why sometimes items show up twice, even if they are installed once
    mods_processed.sort_by(|a, b| a.mod_name.to_lowercase().cmp(&b.mod_name.to_lowercase()));
    mods_processed.dedup_by(|a,b| a.mod_id == b.mod_id);

    Ok(mods_processed)
}