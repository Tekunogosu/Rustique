use std::sync::{Arc, Mutex};
use semver::Version;
use crate::aliases::{DownloadURL, ModVersion};
use crate::api_structs::Releases;
use crate::rustique_errors::RustiqueError;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct LatestVersionFound {
    pub latest_version: Version,
    pub download_url: Option<String>,
}

pub fn parse_latest_version(releases: &Vec<Releases>) -> (ModVersion, DownloadURL) {
    let mut errors :Vec<RustiqueError> = Vec::new();

    let result = releases.iter()
        .filter_map(|release| {
            let version_str = match &release.mod_version {
                Some(v) => v.clone(),
                None => {
                    errors.push(RustiqueError::VersionError {
                        context: format!("{:?} {}", release.filename, ""),
                        source: Version::parse("invalid.version").unwrap_err()
                    });
                    return None;
                }
            };

            // println!("Checking version: {}", version_str);

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

    for error in errors.iter() {
        println!("{}", error.to_string());
    }

    match result {
        Some(v) => (v.latest_version.to_string(), v.download_url.clone().unwrap_or(String::new())),
        None => (String::new(), String::new())
    }
}

pub fn parse_version(mod_version: String) -> Result<Version, RustiqueError> {
    lenient_semver::parse(&mod_version).map_err(|e| RustiqueError::SimpleError(e.to_string()))
}
