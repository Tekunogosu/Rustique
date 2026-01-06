use crate::aliases::{DownloadURL, ModID, ModName, ModVersion};
use crate::api::api_structs::{Mod, ModInfo};
use crate::api::client::{ApiClient};
use crate::api::download::download_requested_mods;
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_zip_metadata, split_modid_version};
use crate::version_management::{parse_latest_version, parse_pinned_version};
use std::collections::{HashMap};
use std::path::PathBuf;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{debug, error, info};
use crate::config::config_manager::{Package, get_config};
use crate::consts::FILE_MODINFO_JSON;
use crate::sync_structs::ModSyncInfo;
use crate::traits::ref_ext::PathRef;
use crate::traits::string_ext::StrLowerExt;

// install & update both will obtain the info needed to fill this struct
#[derive(Debug, Clone, Default)]
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

impl Default for Installed {
    fn default() -> Self {
        Self::new()
    }
}

impl Installed {
    pub fn new() -> Self {
        Self {
            mod_id: String::new(),
            mod_name: String::new(),
            installed_file_path: None,
            old_file_path: None,
            install_version: String::new(),
            success: false,
        }
    }
}


pub async fn install_manager(
    mod_dir: impl PathRef,
    mods_requested: Vec<Install>,
    installed_mods: HashMap<ModID, ModSyncInfo>) -> Result<Vec<Installed>, RustiqueError> {

    let mod_dir = mod_dir.as_ref(); 
    // this is the combined list of all mods installed, once download is completed, now mods will be
    // added here
    let mut total_mods_seen: HashMap<ModID, Installed> = HashMap::with_capacity(installed_mods.len());
    for (mod_id, mod_sync_info) in &installed_mods {
        // this is what is already on the system
        // the version doesn't really matter, we just need to know modid and filepath, which the
        // info from sync would provide that
        let (mod_id, _) = split_modid_version(mod_id);
        total_mods_seen.insert(mod_id.clone(),Installed {
            mod_id: mod_id.clone(),
            mod_name: mod_sync_info.mod_name.clone(),
            installed_file_path: Some(mod_dir.join(mod_sync_info.file_name.clone())),
            success: true,
            old_file_path: Some(mod_dir.join(mod_sync_info.file_name.clone())),
            install_version: mod_sync_info.installed_version.clone(),
        });
    }


    // info!("total_mods_seen: {:#?}", total_mods_seen);
    // info!("mods_requested: {:#?}", mods_requested);

    let client = ApiClient::new();
    let config = get_config().read().await;

    // This vec is filled and then consumed within download_requested_mods
    // each iteration of the loop will add new mods from dependencies to be processed next
    let mut mods_requested = mods_requested.clone();

    // Hold all the mods that were processed during this request.
    // We let the calling function handle what to do with failed installs
    let mut mods_processed: Vec<Installed> = Vec::new();

    let mut passes = 0;
    
    let pb = ProgressBar::new(mods_requested.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise:.cyan}] [{bar:.blue/grey:40}] {pos:.green}/{len:.cyan} {msg:.yellow}")
            .unwrap()
            .progress_chars("█▒░")
    );
    pb.set_message("Downloading...");
    
    loop {

        // this function will consume each value out of the mods_requested so we can rebuild it
        // after the dependencies check
       let recently_installed: Vec<Installed> =  match download_requested_mods(mod_dir, &mut mods_requested, &client, Some(&pb)).await {
            Ok(processed_mods) => {
                debug!("Successfully installed mods: {:?}", processed_mods);
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

        let concurrent_limit = num_cpus::get();

        // clone the keys we need as the async functions won't work with our hashmap, and its cheap to clone hashsets
        let seen_mod_ids: std::collections::HashSet<String> = total_mods_seen.keys().map(|k| k.to_lowercase()).collect();

        #[allow(clippy::redundant_closure)]
        let dep_map: Vec<HashMap<String, String>> = stream::iter(recently_installed.iter())
            .map( |installed_mod| {
                let seen_mod_ids = seen_mod_ids.clone(); // this cheaper than cloning the entire hashmap, logic stays the same
                async move {
                    let path = installed_mod.installed_file_path.clone()?;
                    match extract_zip_metadata::<ModInfo>(&path, FILE_MODINFO_JSON).await {
                        Ok(mod_info) => {
                            let filtered_deps: HashMap<_, _> = mod_info.dependencies
                                .into_iter()
                                .filter(|(dep_id, _)| {
                                    !dep_id.lower_contains("game")
                                        && !dep_id.lower_contains("creative")
                                        && !dep_id.lower_contains("survival")
                                        && !seen_mod_ids.contains(dep_id.to_lowercase().as_str())
                                }).collect();

                            if filtered_deps.is_empty() {
                                None
                            } else {
                                Some(filtered_deps)
                            }
                        },
                        Err(err) => {
                            error!("Failed to extract zip metadata: {:?}", err);
                            None
                        }
                    }
                }
            })
            .buffer_unordered(concurrent_limit)
            .filter_map(|res| futures::future::ready(res))
            .collect()
            .await;


            let mut needed_dependencies: Vec<Install> = dep_map
                .into_iter()
                .flat_map(|deps| deps.into_iter())
                .map(|(mod_id, mod_version)| Install {
                    mod_id,
                    mod_name: String::new(),
                    version_to_install: mod_version,
                    download_url: String::new(),
                    current_file_path: None,
            }).collect();

        passes += 1;
        info!("pass: {}, needed_dependencies : {:?}", passes, needed_dependencies);

        if needed_dependencies.is_empty() {
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
            if let Some(res_mod) =  result.get(mod_to_install.mod_id.as_str()) {
                mod_to_install.mod_name = res_mod.mod_json.name.clone().unwrap_or_default();
                
                let pkg = config.pkg.iter().find(|p| p.mod_id.eq(&res_mod.mod_json.mod_id.to_string()));

                // TODO: add test for this
                let (mod_version, download_url, _,_) = if let Some(mod_pkg) = pkg {
                    parse_pinned_version(&res_mod.mod_json.releases, &mod_pkg.clone(), config.pinned_game_version.clone())
                } else {
                    parse_latest_version(&res_mod.mod_json.releases)
                };
                mod_to_install.download_url = download_url;

                // TODO: needs some refactor on deps resolution because modinfo.json 's dep versions are semver constraints
                // 1. build the version that can suffice the dependency constraints
                // 2. we need to verify for conflicting deps i.e. ModA depends ModB@v1 and ModC depends on ModB@v2
                // we can't install install ModB@v2 because it breaks semver
                // FIXME: should actually resolve the semver constraint, this hack only works for exact match, not for semver constraints
                // (more at: https://github.com/Tekunogosu/Rustique/issues/40)
                // FIXME: this is probably not very rust-way of code
                if mod_to_install.version_to_install == "" {
                mod_to_install.version_to_install = mod_version;
                }
            }
        }
        // increase the total length of the progress bar as there are more things to download
        pb.inc_length(needed_dependencies.len() as u64);
        // seed the mods_requested and go again
        mods_requested.extend(needed_dependencies);
    }

    // TODO: Figure out why sometimes items show up twice, even if they are installed once
    mods_processed.sort_by(|a, b| a.mod_name.to_lowercase().cmp(&b.mod_name.to_lowercase()));
    mods_processed.dedup_by(|a,b| a.mod_id == b.mod_id);

    
    pb.finish_with_message("Finished installing mods");
    
    Ok(mods_processed)
}