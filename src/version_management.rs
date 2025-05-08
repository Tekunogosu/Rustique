use crate::aliases::{DownloadURL, ModID, ModVersion};
use crate::api::api_structs::{Releases};
use crate::rustique_errors::RustiqueError;
use semver::{Version};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::info;


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RustiquePkgs {
    pub mods: HashMap<ModID, RustiquePkgData>,
    pub game_versions: HashSet<String>,
    pub mod_tags: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RustiquePkgData {
    pub pin_version: ModVersion,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct LatestVersionFound {
    pub latest_version: Version,
    pub download_url: Option<String>,
}

pub fn parse_latest_version(releases: &[Releases]) -> (ModVersion, DownloadURL) {
    let mut errors :Vec<RustiqueError> = Vec::new();

    // TODO: Review for version pinning, the error needs to be handled better for that
    let result = releases.iter()
        .filter_map(|release| {
            let version_str = match &release.mod_version {
                Some(v) => v.clone(),
                None => {
                    errors.push(RustiqueError::SimpleError(format!("Unable to parse version NULL for {:?}", release.filename)));
                    return None;
                }
            };

            // only clone when passing to parse_version if required
            match parse_version(version_str.clone()) {
                Ok(version) => Some((version, release.main_file.clone())),
                Err(e) => {
                    errors.push(e);
                    None
                }
            }
        })
        .max_by(|(v1,_), (v2,_)| v1.cmp(v2))
        .map(|(latest_version, download_url)| LatestVersionFound {
            latest_version,
            download_url,
        });

    if !errors.is_empty() {
        for error in errors.iter() {
            info!("{}", error.to_string());
        }
    }

    match result {
        Some(latest_versions_found) => (
            latest_versions_found.latest_version.to_string(),
            latest_versions_found.download_url.clone().unwrap_or_default()),
        None => (String::new(), String::new())
    }
}

pub fn parse_version(mod_version: String) -> Result<Version, RustiqueError> {
    lenient_semver::parse(&mod_version).map_err(|e| RustiqueError::SimpleError(e.to_string()))
}
