use crate::aliases::{DownloadURL, ModID, ModVersion, PinnedVersionInfo};
use crate::api::api_structs::{Releases};
use crate::rustique_errors::RustiqueError;
use semver::{Version};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use owo_colors::OwoColorize;
use tracing::{debug, error, info};
use crate::config::config_manager::Package;
use crate::traits::ref_ext::StrRef;

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
    pub game_versions: Vec<String>,
}

pub fn parse_latest_version(releases: &[Releases]) -> PinnedVersionInfo {
    let mut errors :Vec<RustiqueError> = Vec::new();

    // TODO: Review for version pinning, the error needs to be handled better for that
    let result = releases.iter()
        .filter_map(|release| {
            let Some(version_str) = &release.mod_version else {
                errors.push(RustiqueError::SimpleError(format!("Unable to parse version NULL for {:?}", release.filename)));
                return None;
            };
            
            
            // Check if this mod has a pinned version and return the max by that version

            // only clone when passing to parse_version if required
            match parse_version(&version_str.clone()) {
                Ok(version) => Some((version, release.main_file.clone(), release.tags.clone())),
                Err(e) => {
                    errors.push(e);
                    None
                }
            }
        })
        .max_by(|(v1,_,_), (v2,_,_)| v1.cmp(v2))
        .map(|(latest_version, download_url, game_versions)| LatestVersionFound {
            latest_version,
            download_url,
            game_versions,
        });

    if !errors.is_empty() {
        for error in &errors {
            info!("parse_latest_version: {}", error.to_string());
        }
    }

    
    return_version_results(result)
}

pub fn parse_download_url_from_version<V: AsRef<[Releases]>>(releases: V, version: &str) -> Result<DownloadURL, RustiqueError> {
    releases.as_ref()
        .iter()
        .find_map(|release| {
            release.mod_version.as_ref()
                .filter(|mv| *mv == version)
                .and_then(|_| release.main_file.clone())
        })
        .ok_or_else(|| RustiqueError::SimpleError(format!("Version {version} not found. Use [Rustique info -m modid] for valid versions")))
}


pub fn parse_version(mod_version: &str) -> Result<Version, RustiqueError> {
    lenient_semver::parse(mod_version).map_err(|e| RustiqueError::SimpleError(e.to_string()))
}


/// retrieve a version based on version pinning information. 
pub fn parse_pinned_version(releases: &Vec<Releases>, mod_pkg: &Package, pinned_game_version: impl StrRef) -> PinnedVersionInfo {
    // user should be using Rustique itself to set pinned_game_version so we trust that its valid, otherwise this function
    // will not return the correct version

    let pinned_game_version = pinned_game_version.as_ref();

    // filter out versions that are not declared as compatible with the pinned game version
    let gres = if pinned_game_version.is_empty() {
        info!("pinned_game_version was empty");
        releases.clone()
    } else {
        info!("found pinned_game_version: {pinned_game_version}");
        releases.iter().filter(|r| {
            let mut found = false;
            for tag in &r.tags {
                match compare_versions(tag.as_str(), pinned_game_version) {
                    Ok(c) => match c {
                        std::cmp::Ordering::Less| std::cmp::Ordering::Equal => found = true,
                        std::cmp::Ordering::Greater => {},
                    },
                    Err(e) => {
                        error!("{e}");
                    },
                }
            }

            found

        }).cloned().collect()
    };

    debug!("releases found for game version {:?}",gres);

    // filter out any version that doesnt match the pinned mod version
    let mres = if mod_pkg.pinned_version.is_some() {
        gres.iter().filter(|r| {
            match compare_versions(r.mod_version.clone().unwrap_or_default().as_str(), mod_pkg.pinned_version.clone().unwrap_or_default().as_str()) {
                Ok(c) => match c {
                    std::cmp::Ordering::Less | std::cmp::Ordering::Equal => true,
                    std::cmp::Ordering::Greater => false,
                }
                Err(e) => {
                    info!("{} {}", "parse_pinned_version-mres:".bright_yellow(), e.red().bold());
                    false
                },
            }
        }).cloned().collect()
    } else {
        gres
    };

    let final_res = mres.iter().filter_map(|r| {
        match parse_version(r.mod_version.as_ref().unwrap()) {
            Ok(v) => Some((v, r.main_file.clone(), r.tags.clone())),
            Err(e) => {
                info!("{} {}","parse_pinned_version-final_res:".bright_yellow(), e.red().bold());
                None
            }
        }
    }).max_by(|(v1,_, _),(v2,_,_)| v1.cmp(v2))
      .map(|(latest_version, download_url, game_versions)| LatestVersionFound { 
          latest_version, 
          download_url: download_url.clone(), 
          game_versions 
      });


    return_version_results(final_res)
}

fn return_version_results(result: Option<LatestVersionFound>) -> (ModVersion, DownloadURL, Vec<String>) {
    match result {
        Some(latest_versions_found) => (
            latest_versions_found.latest_version.to_string(),
            latest_versions_found.download_url.clone().unwrap_or_default(),
            latest_versions_found.game_versions
        ),
        None => (String::new(), String::new(), Vec::new())
    }
}

pub fn compare_versions(mod_version: &str, other_version: &str) -> Result<std::cmp::Ordering, RustiqueError> {
    let mv = parse_version(mod_version)?;
    let ov = parse_version(other_version)?;
    Ok(mv.cmp(&ov))
}
