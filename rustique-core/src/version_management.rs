use crate::aliases::{DownloadURL, ModID, ModVersion, PinnedVersionInfo};
use crate::api::api_structs::{Release};
use crate::rustique_errors::RustiqueError;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use owo_colors::OwoColorize;
use tracing::info;
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
    pub changelog: Option<String>,
}

pub fn parse_latest_version(releases: &[Release]) -> PinnedVersionInfo {
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
                Ok(version) => Some((version, release.main_file.clone(), release.tags.clone(), release.changelog.clone())),
                Err(e) => {
                    errors.push(e);
                    None
                }
            }
        })
        .max_by(|(v1,_,_,_), (v2,_,_,_)| v1.cmp(v2))
        .map(|(latest_version, download_url, game_versions, changelog)| LatestVersionFound {
            latest_version,
            download_url,
            game_versions,
            changelog,
        });

    if !errors.is_empty() {
        for error in &errors {
            info!("parse_latest_version: {}", error.to_string());
        }
    }

    
    return_version_results(result)
}

pub fn parse_download_url_from_version<V: AsRef<[Release]>>(releases: V, version: &str) -> Result<DownloadURL, RustiqueError> {
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
pub fn parse_pinned_version(releases: &Vec<Release>, mod_pkg: &Package, pinned_game_version: impl StrRef) -> PinnedVersionInfo {
    // user should be using Rustique itself to set pinned_game_version so we trust that its valid, otherwise this function
    // will not return the correct version

    let pinned_game_version = pinned_game_version.as_ref();

    info!("mod_pkg {:?}", mod_pkg);

    // filter out versions that are not declared as compatible with the pinned game version
    let pinned_game_res = if pinned_game_version.is_empty() {
        info!("pinned_game_version was empty");
        releases.clone()
    } else {
        info!("found pinned_game_version: {pinned_game_version}");
        // let parsed_game_version = parse_version(pinned_game_version)
        //     .map_err(|e| RustiqueError::SimpleError(format!("Failed to parse game version {pinned_game_version} {e}"))).unwrap();

        // let parsed_pinned_game_version = lenient_semver::parse(pinned_game_version).unwrap();

        // info!("Checking parsed pinned game version {parsed_pinned_game_version}");

        releases.iter().filter(|release| {
            info!("Parsing and validating tags {:?}", release.tags);

            let result = release.tags.iter().any( |tag|
                compare_versions(pinned_game_version, tag).unwrap()
            );

            info!("Result from pinned_game_version pares check {result}");
            result
        }).cloned().collect()
    };

    info!("releases found for game version {:?}", pinned_game_res);

    // filter out any version that doesn't match the pinned mod version
    let pinned_mod_res = if mod_pkg.pinned_version.is_some() {
        pinned_game_res.iter().filter(|r| {

            let parsed_mod_version = parse_version(&r.mod_version.clone().unwrap_or(String::from("0.0.0")))
                .map_err(|e| RustiqueError::SimpleError(format!("Failed to parse mod version {e}"))).unwrap();

            VersionReq::parse(&mod_pkg.pinned_version.clone().unwrap_or(String::from("0.0.0")))
                .map_err(|e| RustiqueError::SimpleError(format!("Failed to parse pinned mod version {e}"))).unwrap().matches(&parsed_mod_version)

        }).cloned().collect()
    } else {
        pinned_game_res
    };

    let final_res = pinned_mod_res.iter().filter_map(|r| {
        match parse_version(r.mod_version.as_ref().unwrap()) {
            Ok(v) => Some((v, r.main_file.clone(), r.tags.clone(), r.changelog.clone())),
            Err(e) => {
                info!("{} {}","parse_pinned_version-final_res:".bright_yellow(), e.red().bold());
                None
            }
        }
    }).max_by(|(v1,_,_, _),(v2,_,_,_)| v1.cmp(v2))
                                  .map(|(latest_version, download_url, game_versions, changelog)| LatestVersionFound {
          latest_version, 
          download_url: download_url.clone(), 
          game_versions,
          changelog
      });


    return_version_results(final_res)
}

fn return_version_results(result: Option<LatestVersionFound>) -> (ModVersion, DownloadURL, Vec<String>, String) {
    match result {
        Some(latest_versions_found) => (
            latest_versions_found.latest_version.to_string(),
            latest_versions_found.download_url.clone().unwrap_or_default(),
            latest_versions_found.game_versions,
            latest_versions_found.changelog.unwrap_or(String::new())
        ),
        None => (String::new(), String::new(), Vec::new(), String::new())
    }
}

/*
    The semver library allows for wildcards, and >=, >, <=, <, = to compare versions strings.
    However, it does not allow wildcards IF any -rc/pre/alpha.. exists in the string.

    Example:
        Pinned version 1.22.*

        [Version string 1.22.0
        result: true]

        [Version string 1.22.0-pre.0
        result: false]

    The user can only compare -pre/-rc versions of the same patch level.

    So, pinned >=1.22.0-rc.1  will return false even for 1.22.2-rc.0, despite that version being higher, technically.
    This effectively means that rustique will ONLY download a mod for a stable (meaning non -pre/-rc) of the game,
    unless they explicitly pin the unstable version.

 */
pub fn compare_versions(pinned_version: &str, other_version: &str) -> Result<bool, RustiqueError> {
    // info!("Doing the compare_and_parse_versions");
    let x = VersionReq::parse(pinned_version).map_err(|e| RustiqueError::SimpleError(format!("Failed parsing pinned version in compare_and_parse {e}")))?;
    let y = Version::parse(other_version).map_err(|e| RustiqueError::SimpleError(format!("Failed parsing pinned version in compare_and_parse {e}")))?;

    Ok(x.matches(&y))
}
