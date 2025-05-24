use crate::aliases::{FileName, ModID, UrlString};
use crate::api::api_structs::{GameVersions, Mod, Mods};
use crate::rustique_errors::RustiqueError;
use crate::consts::FILE_MODINFO_JSON;
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use std::fmt::Write;
use clap::ValueEnum;
use reqwest::Response;
use crate::traits::ref_ext::StrRef;

const API_BASE_URL: &str = "https://mods.vintagestory.at/api";
const VS_CDN_STABLE_RELEASE: &str = "https://cdn.vintagestory.at/gamefiles/stable";
const VS_CDN_UNSTABLE_RELEASE: &str = "https://cdn.vintagestory.at/gamefiles/unstable";
const RUSTIQUE_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"), "  (github: Tekunogosu/Rustique)");

#[derive(Debug, Clone)]
pub struct ApiClient {
    agent: Arc<reqwest::Client>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum VSMirrorType {
    Stable,
    Unstable
}

#[derive(Debug, Clone, ValueEnum)]
pub enum VSExecutabletype {
    Server, 
    Client
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, ValueEnum, Debug)]
pub enum VSOSType {
    Linux,
    OSX,
    Windows
}

impl Display for VSOSType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VSOSType::Linux => write!(f, "linux"),
            VSOSType::OSX => write!(f, "osx"),
            VSOSType::Windows => write!(f, "windows")
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum VSWinInstallerType {
    Install,
    Update
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            agent: Arc::new(
                reqwest::Client::builder()
                    .timeout(Duration::from_secs(20))
                    .user_agent(RUSTIQUE_USER_AGENT)
                    .build()
                    .expect("Failed to build HTTP client")
            ),
        }
    }

    pub fn _with_agent(agent: Arc<reqwest::Client>) -> Self {
        Self { agent }
    }

    pub fn api_uri(endpoint: &str) -> String {
        format!("{API_BASE_URL}/{endpoint}")
    }
    fn cdn_uri_stable(endpoint: &str) -> String { format!("{VS_CDN_STABLE_RELEASE}/{endpoint}")}
    fn cdn_uri_unstable(endpoint: &str) -> String { format!("{VS_CDN_UNSTABLE_RELEASE}/{endpoint}") }

    pub async fn fetch_all_mods(&self) -> Result<Mods, RustiqueError> {
        let response = self.agent.get(Self::api_uri("mods"))
            .send()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: "fetch_all_mods (get): ".to_string(),
                source: e,
            })?;

        response.json::<Mods>()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: "fetch_all_mods (json): ".to_string(),
                source: e,
            })
    }

    pub async fn fetch_mod(&self, mod_id: impl StrRef) -> Result<Mod, RustiqueError> {
        let mod_id = mod_id.as_ref();
        if mod_id.is_empty() {
            error!("Mod id is empty {}", mod_id);
            return Err(RustiqueError::MalformedModInfoJson("The mod id received was empty.. unable to download whatever mod this is.".to_string()));
        }

        info!("{} {}", "Fetching mod: ".bright_green(), mod_id.bright_yellow());

        let response = self.agent.get(Self::api_uri(&format!("mod/{mod_id}")))
            .send()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: format!("fetch_mod (get) [{mod_id}]"),
                source: e
            })?;


        response.json::<Mod>()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: format!("fetch_mod (json) [{mod_id}] - They may have provided the wrong mod_id or the api is not responding. Retry, if it fails you will need to manually update the mod."),
                source: e
            })
    }

    pub async fn fetch_mods_parallel(&self, mod_list: Vec<ModID>) -> Result<HashMap<ModID, Mod>, RustiqueError> {
        // Create a vector to hold all our task handles
        let mut tasks = Vec::with_capacity(mod_list.len());

        // Spawn a task for each mod
        for mod_id in mod_list {
            info!("ModID: {}", mod_id);

            if mod_id.is_empty() {
                error!("\n\r\tModID is empty or missing mod_id. Please contact the author to correct their malformed {FILE_MODINFO_JSON}.\n\r\tWithout the mod id, Rustique will be unable to manage this mod.");
                continue;
            }

            let client = self.clone();

            // Spawn an async task for this mod
            let task = tokio::spawn(async move {
                match client.fetch_mod(&mod_id).await {
                    Ok(the_mod) => {
                        Some((mod_id, the_mod))
                    },
                    Err(e) => {
                        eprintln!("{mod_id} {e}");
                        None
                    }
                }
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete and collect results
        let mut results = HashMap::new();
        for task in tasks {
            // Handle any JoinError from the task itself
            if let Ok(Some((mod_id, the_mod))) = task.await {
                results.insert(mod_id.into(), the_mod);
            }
        }

        Ok(results)
    }

    pub async fn fetch_game_versions(&self) -> Result<HashSet<String>, RustiqueError> {
        let res = self.agent.get(Self::api_uri("gameversions"))
            .send().await
            .map_err(|e| RustiqueError::ApiError {
            context: "Failed during gameversions api call".to_string(),
            source: e,
        })?;
       
        let versions = res.json::<GameVersions>().await.map_err(|e| RustiqueError::ApiError {
            context: "Failed parsing game versions api data".to_string(),
            source: e,
        })?;
        
        let hash: HashSet<String> = versions.game_versions
            .iter().map(|gv| &gv.name).cloned().collect();
        
        Ok(hash)
    }

    pub async fn get_request(&self, mod_uri: &str) -> Result<Response, RustiqueError> {
        self.agent.get(mod_uri)
            .send()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: format!("get_request: {mod_uri}"),
                source: e,
            })
    }
    
    pub fn download_uri(
        &self,
        os_type: &VSOSType, 
        exe_type: &VSExecutabletype, 
        vsmirror_type: &VSMirrorType, 
        game_version: &str,
        win_installer: Option<&VSWinInstallerType>
    ) -> Result<(UrlString, FileName), RustiqueError> {
        
        let mut download_str = String::from("vs_");
        
        let etype = match exe_type {
            VSExecutabletype::Client => "client",
            VSExecutabletype::Server => "server",
        };
        
        if matches!(os_type, VSOSType::Windows) {
            if etype == "server" {
                download_str += "server_win";
            } else {
                download_str += match win_installer {
                    Some(VSWinInstallerType::Install) | None => "install_",
                    Some(VSWinInstallerType::Update) => "update_",
                };
                download_str += "win";
            }
        } else {
            write!(&mut download_str, "{}_{}", etype, os_type.to_string().as_str()).map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        }
       
        // use std::fmt::write to avoid extra allocation with format!
        write!(&mut download_str, "-x64_{game_version}").map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        
        download_str += if matches!(os_type, VSOSType::OSX) || matches!(os_type, VSOSType::Linux) {
            ".tar.gz"
        } else if matches!(exe_type, VSExecutabletype::Server) {
            ".zip"  
        } else {
            ".exe"
        };
        
        
        let cdn = if matches!(vsmirror_type, VSMirrorType::Stable) {
            Self::cdn_uri_stable(&download_str)
        } else {
            Self::cdn_uri_unstable(&download_str)
        };
        
        Ok((cdn, download_str))
    }
    
    pub async fn head(&self, uri: &str) -> Result<Response, RustiqueError> {
        self.agent.head(uri).send().await.map_err(|e| RustiqueError::ApiError {
            context: format!("Failed calling agent.head({uri})"),
            source: e,
        })
    }
}