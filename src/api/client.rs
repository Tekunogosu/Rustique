use crate::aliases::{ModID, ModName};
use crate::api::api_structs::{Mod, Mods};
use crate::rustique_errors::RustiqueError;
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

const API_BASE_URL: &str = "https://mods.vintagestory.at/api";
const RUSTIQUE_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"), "  (github: Tekunogosu/Rustique)");

#[derive(Debug, Clone)]
pub struct ApiClient {
    agent: Arc<reqwest::Client>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ModApiFetch {
    pub mod_id: ModID,
    pub mod_name: ModName,
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

    fn uri(endpoint: &str) -> String {
        format!("{API_BASE_URL}/{endpoint}")
    }

    pub async fn fetch_all_mods(&self) -> Result<Mods, RustiqueError> {
        let response = self.agent.get(Self::uri("mods"))
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

    pub async fn fetch_mod(&self, mod_id: &str) -> Result<Mod, RustiqueError> {
        if mod_id.is_empty() {
            error!("Mod id is empty {}", mod_id);
            return Err(RustiqueError::MalformedModInfoJson("The mod id received was empty.. unable to download whatever mod this is.".to_string()));
        }

        info!("{} {}", "Fetching mod: ".bright_green(), mod_id.bright_yellow());

        let response = self.agent.get(Self::uri(&format!("mod/{mod_id}")))
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
                error!("\n\r\tModID is empty or missing mod_id. Please contact the author to correct their malformed modinfo.json.\n\r\tWithout the mod id, Rustique will be unable to manage this mod.");
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
                results.insert(mod_id, the_mod);
            }
        }

        Ok(results)
    }

    // pub fn _fetch_game_versions(&self) -> Result<HashSet<String>, RustiqueError> {
    //     Ok(HashSet::new())
    // }

    pub async fn get_request(&self, mod_uri: &str) -> Result<reqwest::Response, RustiqueError> {
        self.agent.get(mod_uri)
            .send()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: format!("get_request: {mod_uri}"),
                source: e,
            })
    }
}