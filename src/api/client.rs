use crate::aliases::{FileName, ModID, UrlString};
use crate::api::api_structs::{GameVersions, Mod, Mods};
use crate::rustique_errors::RustiqueError;
use crate::consts::FILE_MODINFO_JSON;
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};
use std::fmt::Write;
use clap::ValueEnum;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Response;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use crate::traits::ref_ext::StrRef;

const API_BASE_URL: &str = "https://mods.vintagestory.at/api";
const VS_CDN_STABLE_RELEASE: &str = "https://cdn.vintagestory.at/gamefiles/stable";
const VS_CDN_UNSTABLE_RELEASE: &str = "https://cdn.vintagestory.at/gamefiles/unstable";
pub const RUSTIQUE_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"), "  (github: Tekunogosu/Rustique)");

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
                context: format!("fetch_mod (get) [{mod_id}]", ),
                source: e
            })?;

        let headers = response.headers().clone();
        let status_code = response.status();
        // let res_text = response.text().map_err(|e| RustiqueError::ApiError {
        //     context: format!("fetch_mod (json) [{mod_id}]: failed to get response text"),
        //     source: e,
        // })?;
        

        info!("fetch_mod ({}): Status Code: {}", mod_id.magenta(), status_code.magenta());
        info!("fetch_mod ({}): Headers: {:?}", mod_id.magenta(), headers.bright_blue());

        let text = response.text().await
        .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        
        
        
        let parsed: Mod = serde_json::from_str(&text).map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        debug!("Parsed {:?}", parsed);

        Ok(parsed)
    }

    pub async fn fetch_mods_parallel(&self, mod_list: Vec<ModID>) -> Result<HashMap<ModID, Mod>, RustiqueError> {

        let valid_ids: Vec<ModID> = mod_list.into_iter().filter(|m| {
            if m.is_empty() {
               error!("\n\r\tModID is empty or missing mod_id. Please contact the author to correct their malformed {FILE_MODINFO_JSON}.\n\r\tWithout the mod id, Rustique will be unable to manage this mod.");
               false
            } else {
                true
            }
        }).collect();


        if valid_ids.is_empty() {
            return Ok(HashMap::new())
        }

        // progress bar for completed calls
        let pb = ProgressBar::new(valid_ids.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise:.cyan}] [{bar:.cyan/grey:40}] {pos:.green}/{len:.cyan} {msg:.yellow}")
                .unwrap()
                .progress_chars("█▒░")
        );
        pb.set_message("Fetching mods...");

        // Create a vector to hold all our task handles
        let mut tasks: Vec<JoinHandle<Option<(ModID, Mod)>>> = Vec::with_capacity(valid_ids.len());

        // Spawn a task for each mod
        for (i, mod_id) in valid_ids.into_iter().enumerate() {
            info!("ModID: {}", mod_id);

            let client = self.clone();
            let pb_clone = pb.clone();
            // Spawn an async task for this mod
            let task = tokio::spawn(async move {
                match client.fetch_mod(&mod_id).await {
                    Ok(the_mod) => {
                        pb_clone.set_message(mod_id.to_string());
                        pb_clone.inc(1);
                        Some((mod_id, the_mod))
                    },
                    Err(e) => {
                        info!("{mod_id} {e}");
                        pb_clone.set_message(format!("Failed: {}", mod_id.red()));
                        pb_clone.inc(1);
                        None
                    }
                }
            });

            tasks.push(task);

            // slight pause every 10 requests to not overwhelm the api
            if i % 10 == 9 {
                sleep(Duration::from_millis(100)).await;
            }
        }

        let results_vec = join_all(tasks).await;
        // Wait for all tasks to complete and collect results
        let mut results = HashMap::new();
        for (mod_id, mod_info) in results_vec.into_iter().flatten().flatten() {
            // Handle any JoinError from the task itself
            results.insert(mod_id, mod_info);
        }

        pb.finish_with_message("Fetch Complete");
        Ok(results)
    }

    pub async fn fetch_game_versions(&self) -> Result<HashSet<String>, RustiqueError> {
        let response = self.agent.get(Self::api_uri("gameversions"))
                           .send().await
                           .map_err(|e| RustiqueError::ApiError {
            context: "Failed during gameversions api call".to_string(),
            source: e,
        })?;
        
        
        let text = response.text().await.map_err(|e| RustiqueError::ApiError {
            context: "Failed parsing game versions api data".to_string(),
            source: e,
        })?;
       
        let versions: GameVersions = serde_json::from_str(&text).map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        
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