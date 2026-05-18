use crate::aliases::{DownloadURL, ModID, ModName, ModVersion};
use crate::api::api_structs::{Mod, ModInfo};
use crate::api::client::{ApiClient};
use crate::api::download::download_requested_mods;
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_zip_metadata, split_modid_version};
use crate::version_management::{parse_latest_version, parse_pinned_version};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use comfy_table::{Attribute, Color};
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{debug, error, info};
use crate::config::config_manager::get_config;
use crate::consts::FILE_MODINFO_JSON;
use crate::information_utils::notice;
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


#[derive(Debug, Clone)]
pub struct Requester {
    pub mod_id: ModID,
    pub required_version: ModVersion,
}

#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub mod_id: ModID,
    pub mod_name: ModName,
    pub version_to_install: ModVersion,
    pub download_url: DownloadURL,
    pub requesters: Vec<Requester>,
}

pub type DependencyGraph = HashMap<ModID, ResolvedDep>;

pub async fn resolve_dependencies(
    mod_dir: &std::path::Path,
    initial_mods: Vec<Install>,
    installed_mods: &BTreeMap<ModID, ModSyncInfo>,
    client: &ApiClient,
    pb: &ProgressBar,
) -> Result<(DependencyGraph, Vec<Installed>), RustiqueError> {
    let config = get_config().read().await;
    let mut graph: DependencyGraph = HashMap::new();
    let mut queue: VecDeque<Install> = VecDeque::new();
    let mut all_installed: Vec<Installed> = Vec::new();

    // seed graph with already-installed mods so BFS skips them
    for (mod_id, sync_info) in installed_mods {
        let (mod_id, _) = split_modid_version(mod_id);
        graph.insert(mod_id.to_lowercase(), ResolvedDep {
            mod_id: mod_id.clone(),
            mod_name: sync_info.mod_name.clone(),
            version_to_install: sync_info.installed_version.clone(),
            download_url: String::new(),
            requesters: vec![Requester {
                mod_id: String::from("installed"),
                required_version: sync_info.installed_version.clone(),
            }],
        });
    }

    // seed queue with initially requested mods
    // so they are "seen" before the first download pass
    for install in initial_mods {
        let key = install.mod_id.to_lowercase();
        if !graph.contains_key(&key) {
            graph.insert(key, ResolvedDep {
                mod_id: install.mod_id.clone(),
                mod_name: install.mod_name.clone(),
                version_to_install: install.version_to_install.clone(),
                download_url: install.download_url.clone(),
                requesters: vec![Requester {
                    mod_id: String::from("user"),
                    required_version: install.version_to_install.clone(),
                }],
            });
            queue.push_back(install);
        }
    }

    let concurrent_limit = num_cpus::get();

    while !queue.is_empty() {
        let mut batch: Vec<Install> = queue.drain(..).collect();

        let recently_installed = download_requested_mods(mod_dir, &mut batch, client, Some(pb))
            .await
            .unwrap_or_else(|err| {
            error!("Failed to install batch: {:?}", err);
            Vec::new()
        });
        all_installed.extend(recently_installed.clone());

        // read modinfo.json from each downloaded mod to discover dependencies
        #[allow(clippy::redundant_closure)]
        let dep_maps: Vec<HashMap<String, String>> = stream::iter(recently_installed.iter())
            .map(|installed_mod| {
                async move {
                    let path = installed_mod.installed_file_path.clone()?;
                    match extract_zip_metadata::<ModInfo>(&path, FILE_MODINFO_JSON).await {
                        Ok(mod_info) => {
                            let deps: HashMap<_, _> = mod_info.dependencies
                                .into_iter()
                                .filter(|(dep_id, _)| {
                                    !dep_id.lower_contains("game")
                                        && !dep_id.lower_contains("creative")
                                        && !dep_id.lower_contains("survival")
                                })
                                .collect();
                            if deps.is_empty() { None } else { Some(deps) }
                        }
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

        // flatten all deps from every mod in the batch into a single HashMap
        // hashmap collects deduplicates by key, so shared deps across mods only appear once
        // filter already-seen mods (those in graph) before collecting
        let new_deps: HashMap<String, String> = dep_maps
            .into_iter()
            .flat_map(|deps| deps.into_iter())
            .filter(|(dep_id, _)| !graph.contains_key(&dep_id.to_lowercase()))
            .collect();

        if new_deps.is_empty() {
            continue;
        }

        let mod_ids: Vec<ModID> = new_deps.keys().cloned().collect();
        let api_results: HashMap<ModID, Mod> = client.fetch_mods_parallel(mod_ids).await?;

        for (dep_id, required_version) in &new_deps {
            let key = dep_id.to_lowercase();
            if graph.contains_key(&key) {
                continue;
            }

            if let Some(api_mod) = api_results.get(dep_id.as_str()) {
                let mod_name = api_mod.mod_json.name.clone().unwrap_or_default();
                // println!("Mod name {mod_name}");
                // the url alias IS the mod ID in MOST cases. Need a check for validity
                let mod_id = if let Some(mod_alias) = &api_mod.mod_json.url_alias {
                    mod_alias
                } else  {
                    &api_mod.mod_json.mod_id.clone().to_string()
                };

                let pkg = config.pkg.iter().find(|p| p.mod_id.eq(mod_id));

                let (version, url, _, _) = if let Some(mod_pkg) = pkg {
                    // println!("Parse_pinned_version {:?}", mod_pkg);
                    match parse_pinned_version(&api_mod.mod_json.releases, &mod_pkg.clone(), config.pinned_game_version.as_str(), config.allow_unstable) {
                        Ok(pv) => pv,
                        Err(e) => {
                            notice(format!("Unable to locate compatible versions for {} -- {}", dep_id, e), Some(Color::Red), vec![Attribute::Bold]);
                            continue;
                        }
                    }
                } else {
                    // println!("parse_latest_version");
                    parse_latest_version(&api_mod.mod_json.releases)
                };

                // println!("Trying to download {} from {}", version, url);

                // insert into graph BEFORE pushing to queue — this prevents a dep discovered
                // by two mods in the same batch from being queued twice
                graph.insert(key, ResolvedDep {
                    mod_id: dep_id.clone(),
                    mod_name: mod_name.clone(),
                    version_to_install: version.clone(),
                    download_url: url.clone(),
                    requesters: vec![Requester {
                        mod_id: String::from("dependency"),
                        required_version: required_version.clone(),
                    }],
                });

                pb.inc_length(1);
                queue.push_back(Install {
                    mod_id: dep_id.clone(),
                    mod_name,
                    version_to_install: version,
                    download_url: url,
                    current_file_path: None,
                });
            }
        }
    }

    Ok((graph, all_installed))
}

pub async fn install_manager(
    mod_dir: impl PathRef,
    mods_requested: Vec<Install>,
    installed_mods: BTreeMap<ModID, ModSyncInfo>) -> Result<Vec<Installed>, RustiqueError> {

    let mod_dir = mod_dir.as_ref();
    let client = ApiClient::new();

    let pb = ProgressBar::new(mods_requested.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise:.cyan}] [{bar:.blue/grey:40}] {pos:.green}/{len:.cyan} {msg:.yellow}")
            .unwrap()
            .progress_chars("█▒░")
    );
    pb.set_message("Downloading...");

    let (_, mut mods_processed) = resolve_dependencies(mod_dir, mods_requested, &installed_mods, &client, &pb).await?;

    mods_processed.sort_by(|a, b| a.mod_name.to_lowercase().cmp(&b.mod_name.to_lowercase()));

    pb.finish_with_message("Finished installing mods");

    Ok(mods_processed)
}