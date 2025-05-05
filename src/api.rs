use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use colored::Colorize;
use ureq::{Agent, Body, Error};
use rayon::prelude::*;
use tracing::{debug, error, info};
use ureq::config::Config;
use ureq::http::Response;
use crate::api_structs::{Mod, ModInfo, Mods};
use crate::rustique_errors::RustiqueError;

const API_BASE_URL: &str = "https://mods.vintagestory.at/api";
const RUSTIQUE_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"), "  (github: Tekunogosu/Rustique)");

#[derive(Debug, Clone)]
pub struct ApiClient {
    agent: Arc<reqwest::Client>,
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

    pub fn with_agent(agent: Arc<reqwest::Client>) -> Self {
        Self { agent }
    }

    fn uri(&self, endpoint: &str) -> String {
        format!("{}/{}", API_BASE_URL, endpoint)
    }

    pub async fn fetch_all_mods(&self) -> Result<Mods, RustiqueError> {
        let response = self.agent.get(&self.uri("mods"))
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
            return Err(RustiqueError::MalformedModInfoJson(format!("{}","The mod id received was empty.. unable to download whatever mod this is.")));
        }

        info!("{} {}", "Fetching mod: ".bright_green(), mod_id.bright_yellow());

        let response = self.agent.get(&self.uri(&format!("mod/{}", mod_id)))
            .send()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: format!("fetch_mod (get) [{}]", mod_id),
                source: e
            })?;

        response.json::<Mod>()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: format!("fetch_mod (json) [{}]", mod_id),
                source: e
            })
    }

    pub async fn fetch_mods_parallel(&self, mod_list: Vec<ModInfo>) -> Result<HashMap<String, Mod>, RustiqueError> {
        // Create a vector to hold all our task handles
        let mut tasks = Vec::with_capacity(mod_list.len());

        // Spawn a task for each mod
        for mod_info in mod_list {
            if mod_info.mod_id.is_empty() {
                error!("\n\r\tMod {}: Has an empty or missing mod_id. Please contact the author to correct their malformed modinfo.json.\n\r\tWithout the mod id, Rustique will be unable to manage this mod.", mod_info.name.red().bold());
                continue;
            }

            let client = self.clone();
            let mod_id = mod_info.mod_id.clone();

            // Spawn an async task for this mod
            let task = tokio::spawn(async move {
                match client.fetch_mod(&mod_id).await {
                    Ok(the_mod) => {
                        Some((mod_id, the_mod))
                    },
                    Err(e) => {
                        eprintln!("{} {}", mod_id, e);
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

    pub async fn fetch_game_versions(&self) -> Result<HashSet<String>, RustiqueError> {
        Ok(HashSet::new())
    }

    pub async fn get_request(&self, mod_uri: &str) -> Result<reqwest::Response, RustiqueError> {
        self.agent.get(mod_uri)
            .send()
            .await
            .map_err(|e| RustiqueError::ApiError {
                context: format!("get_request: {}", mod_uri),
                source: e,
            })
    }
}

//
// #[derive(Debug, Clone)]
// pub struct ApiClient {
//     agent: Arc<Agent>,
// }
//
// impl ApiClient {
//     pub fn new() -> Self {
//         Self {
//             agent: Arc::new(
//                 Agent::new_with_config(
//                     Config::builder()
//                         .timeout_global(Some(Duration::from_secs(20)))
//                         .user_agent(RUSTIQUE_USER_AGENT)
//                         .build()
//                 )
//             ),
//         }
//     }
//
//     pub fn with_agent(agent: Arc<Agent>) -> Self {
//         Self { agent }
//     }
//
//     fn uri(&self, endpoint: &str) -> String {
//         format!("{}/{}", API_BASE_URL, endpoint)
//     }
//
//     pub fn fetch_all_mods(&self) -> Result<Mods, RustiqueError> {
//         self.agent.get(&self.uri("mods")).call().map_err(|e| RustiqueError::ApiError {
//             context: "fetch_all_mods (get): ".to_string(),
//             source: e,
//         })?.body_mut().read_json::<Mods>().map_err(|e| RustiqueError::ApiError {
//             context: "fetch_all_mods (json): ".to_string(),
//             source: e,
//         })
//     }
//
//     pub fn fetch_mod(&self, mod_id: &str) -> Result<Mod, RustiqueError> {
//         if mod_id.is_empty() {
//             error!("Mod id is empty {}", mod_id);
//             return Err(RustiqueError::MalformedModInfoJson(format!("{}","The mod id received was empty.. unable to download whatever mod this is.")));
//         }
//
//         info!("{} {}", "Fetching mod: ".bright_green(), mod_id.bright_yellow());
//
//         self.agent.get(&self.uri(&format!("mod/{}", mod_id))).call().map_err(|e| RustiqueError::ApiError {
//             context: format!("fetch_mod (get) [{}]", mod_id),
//             source: e
//         })?.body_mut().read_json::<Mod>().map_err(|e| RustiqueError::ApiError {
//             context: format!("fetch_mod (json) [{}]", mod_id),
//             source: e
//         })
//     }
//
//     pub fn fetch_mods_parallel(&self, mod_list: Vec<ModInfo>) -> Result<HashMap<String, Mod>, RustiqueError> {
//         // let client = Arc::new(self);
//
//          let result = mod_list
//              .par_iter()
//              .filter_map(|mod_info| {
//                  debug!("{:#?}",mod_info);
//
//                  if mod_info.mod_id.is_empty() {
//                      error!("\n\r\tMod {}: Has an empty or missing mod_id. Please contact the author to correct their malformed modinfo.json.\n\r\tWithout the mod id, Rustique will be unable to manage this mod.", mod_info.name.red().bold());
//                      return None;
//                  }
//
//                  match self.fetch_mod(mod_info.mod_id.as_ref()) {
//                      Ok(the_mod) => {
//                          Some((mod_info.mod_id.clone(), the_mod))
//                      },
//                      Err(e) => {
//                          eprintln!("{} {}", mod_info.mod_id, e);
//                          None
//                      }
//                  }
//              }).collect();
//
//         Ok(result)
//     }
//
//     pub fn fetch_game_versions(&self) -> Result<HashSet<String>, RustiqueError> {
//
//         Ok(HashSet::new())
//     }
//
//     pub fn get_request(&self, mod_uri: &str) -> Result<Response<Body>, RustiqueError> {
//         self.agent.get(mod_uri).call().map_err(|e| RustiqueError::ApiError {
//             context: format!("get_request: {}",mod_uri),
//             source: e,
//         })
//     }
// }
