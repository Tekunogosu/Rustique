use crate::aliases::{DownloadURL, ModID, ModVersion, PinnedVersionInfo};
use crate::api::api_structs::{Release};
use crate::rustique_errors::RustiqueError;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use owo_colors::OwoColorize;
use tracing::{debug, info};
use crate::config::config_manager::Package;

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

// TODO: Needs to follow the config.allow_unstable option just as the parse_pinned_version method does.
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


pub fn parse_pinned_version(mod_releases: &[Release], mod_pkg: &Package, pinned_game_version: &str, allow_unstable: bool) -> Result<PinnedVersionInfo, RustiqueError> {


    // VersionReq pinned_game_version and the mod_pkg.pinned_version so it doesn't have to be parsed repeatedly when checking
    // the mod_releases

    let mut check_pinned_game_version= false;
    let mut check_pinned_mod_version = false;

    let parsed_pinned_game_version = if !pinned_game_version.is_empty() {
        match VersionReq::parse(pinned_game_version) {
            Ok(v) => {
                check_pinned_game_version = true;
                v
            },
            Err(e) => {
                return Err(RustiqueError::SimpleError(format!("Pinned Game Version ({}) parsing error {}", pinned_game_version, e)));
            }
        }
    } else {
        VersionReq::default()
    };

    let parsed_pinned_mod_version = if let Some(mpv) = &mod_pkg.pinned_version {
        match VersionReq::parse(mpv) {
            Ok(v) => {
                check_pinned_mod_version = true;
                v
            } ,
            Err(e) => {
               return Err(RustiqueError::SimpleError(format!("Pinned mod Version ({}) parsing error {}", mpv, e)));
            }
        }
    } else {
        VersionReq::default()
    };


    println!("Pinned game version to check {parsed_pinned_game_version}");
    println!("Pinned mod version to check {parsed_pinned_mod_version}");

    // filter once
    // iterate through releases
    // check if release is valid against the pinned game version AND against the pinned mod version from the mod_config_pkg
    let compatible_releases : Vec<Release> =  mod_releases.iter().filter(|release| {
        // check for pinned game version compatibility

        let compatible_with_pinned_game_version = if check_pinned_game_version {
            find_compatible_versions(&parsed_pinned_game_version, release.tags.clone(), allow_unstable).unwrap_or(false)
        } else { true };

        let compatible_with_pinned_mod_version = if check_pinned_mod_version {
            find_compatible_versions(&parsed_pinned_mod_version, vec![release.mod_version.clone().unwrap_or("0.0.0".into())], allow_unstable).unwrap_or(false)
        } else { true };

        compatible_with_pinned_game_version && compatible_with_pinned_mod_version
        // check for pinned mod version compatibility
    }).cloned().collect();

    info!("compatible_releases: {:?}", compatible_releases);

    if compatible_releases.is_empty() {
        return Err(RustiqueError::NoVersionFound("Compatible versions not found based on pinned conditions".into()))
    }


    let final_res = compatible_releases.iter().filter_map(|r| {
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
          download_url,
          game_versions,
          changelog
      });

    Ok(return_version_results(final_res))
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

    This method attempts to work around this limitation when comparing version strings by stripping
    the pre-release off (assuming allow_unstable is set in the config) and comparing the major.minor.patch
    only.

 */
fn find_compatible_versions(parsed_condition: &VersionReq, versions_to_check: Vec<String>, allow_unstable: bool) -> Result<bool, RustiqueError> {

    // if user installs a mod with modid@version, VersionReq treats a version 1.2.3 the same as >=1.2.3
    // This causes a later version to be installed

    // version is compatible if there is no pinned_condition
    // if pinned_condition.is_empty() { return Ok(true); }
    //
    // let parsed_condition = match VersionReq::parse(pinned_condition) {
    //     Ok(v) => v,
    //     Err(e) => {
    //         return Err(RustiqueError::SimpleError(format!("Pinned condition ({}) parsing error {}", pinned_condition, e)));
    //     }
    // };

    let matches: Vec<Version> = versions_to_check.iter().filter_map(|v_str| {
        let parsed_v = match lenient_semver::parse(v_str) {
            Ok(v) => v,
            Err(_) => {
                println!("{} is not a valid semver format", v_str);
                return None
            },
        };

        if !parsed_v.pre.is_empty() && !allow_unstable {
            debug!("Skipping {parsed_v}");
            return None
        }

        let check_version = if !parsed_v.pre.is_empty() && allow_unstable {
            // strip the pre-releases to check against major.minor.patch
            match Version::parse(&format!("{}.{}.{}", parsed_v.major, parsed_v.minor, parsed_v.patch)) {
                Ok(stripped) => stripped,
                Err(_) => return None, // Shouldn't happen as the original was parsed successfully,
            }
        } else {
            parsed_v.clone()
        };


        if parsed_condition.matches(&check_version) {
            Some(parsed_v)
        } else { None }
    }).collect();

    // grab only the latest version IF more than 1 matches.
    // easy if allow_unstable == false

    let matched = match matches.iter().max() {
        Some(m) => {
            info!("Matched: {m}");
            format!("{m}")
        },
        None => {
            debug!("No valid version found for condition {}", parsed_condition);
            String::new()
        }
    };

    Ok(!matched.is_empty())
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






pub fn compare_versions(pinned_version: &str, other_version: &str) -> Result<bool, RustiqueError> {
    // info!("Doing the compare_and_parse_versions");
    let x = VersionReq::parse(pinned_version).map_err(|e| RustiqueError::SimpleError(format!("Failed parsing pinned version in compare_and_parse {e}")))?;
    let y = Version::parse(other_version).map_err(|e| RustiqueError::SimpleError(format!("Failed parsing pinned version in compare_and_parse {e}")))?;

    Ok(x.matches(&y))
}
