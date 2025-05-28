use std::env::temp_dir;
use tokio::fs::File;
use std::sync::Arc;
use std::time::Duration;
use comfy_table::{Attribute, CellAlignment, Color};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use reqwest::header::ACCEPT;
use self_update::backends::github::Update;
use self_update::cargo_crate_version;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};
use uuid::Uuid;
use crate::api::client::{ApiClient, RUSTIQUE_USER_AGENT};
use crate::commands::download::download_file;
use crate::information_utils::{notice, rustique_message, CellData, RustiqueMessage};
use crate::rustique_errors::RustiqueError;
use crate::updater::github_api_args::GithubReleases;
use crate::version_management::parse_version;

// this url shows all releases for rustique published to github
const GITHUB_RUSTIQUE_URI: &str = "https://api.github.com/repos/Tekunogosu/Rustique/releases";

pub struct GithubApi {
    agent: Arc<reqwest::Client>,
}

impl GithubApi {
    pub fn new() -> Self {
        Self {
            agent: Arc::new(
                Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent(RUSTIQUE_USER_AGENT)
                .build().expect("Failed to build Github API client")
            )
        }
    }

    pub fn api_url(endpoint: &str) -> String {
        format!("{GITHUB_RUSTIQUE_URI}/{endpoint}")
    }

    pub async fn get_latest_release(&self) -> Result<GithubReleases, RustiqueError> {
        let uri = Self::api_url("latest");
        info!("URL: {}", &uri);
        let response= self.agent.get(uri)
            .header(ACCEPT, "application/vnd.github+json")
            .send().await.map_err(|e| RustiqueError::SimpleError(format!("get_latest_release: {e}")))?;

        let text = response.text().await.map_err(|e| RustiqueError::SimpleError(format!("get_latest_release: txt {e}")))?;

        debug!("get_latest_release: txt: {text}");

        let parsed: GithubReleases = serde_json::from_str(&text)
            .map_err(|e| RustiqueError::SimpleError(format!("get_latest_release: (json) {e}")))?;

        Ok(parsed)
    }
}

pub async fn check_for_update(hide_message: bool) -> Result<bool, RustiqueError> {

    let client = GithubApi::new();

    let latest_release = client.get_latest_release().await?;

    let latest_version = parse_version(latest_release.tag_name.as_str())?;
    let current_version = parse_version(cargo_crate_version!())?;
    // let current_version = parse_version("0.4.2")?;

    let has_update = latest_version > current_version;

    if !hide_message {
        if has_update {
            rustique_message(RustiqueMessage {
                header: Some(CellData::new("Rustique Update Available!".into(), Some(Color::Green), vec![Attribute::Bold], Some(CellAlignment::Center))),
                message: vec![
                    CellData::new(format!("Version: {latest_version} is now available!"), Some(Color::Green), vec![Attribute::Bold], Some(CellAlignment::Center)),
                    CellData::new("You can update Rustique with the following command: ".into(), Some(Color::Yellow), vec![], Some(CellAlignment::Center)),
                    CellData::new("./Rustique self update".into(), Some(Color::Magenta), vec![Attribute::Bold], Some(CellAlignment::Center)),
                    CellData::default(),
                    CellData::new("You can disable this message with ./Rustique config set --disable-update-message".into(), Some(Color::Cyan), vec![Attribute::Italic, Attribute::Dim], Some(CellAlignment::Center)),
                ],
            });
        } else {
            notice("Rustique is up-to-date!", Some(Color::Green), vec![Attribute::Bold]);
        }
    }

    info!("Current Version: {current_version}, latest version {latest_version}");

    Ok(has_update)
}

pub async fn self_update_binary(force_update: bool) -> Result<(), RustiqueError> {

    // get latest release based in arch
    // download it to a temp dir
    // unzip the file
    // copy the current binary to tmp dir and put the new binary in its place, with the same name and permissions
    // if file swap failed, revert changes.. move old binary back in place, clean up tmp download
    // if success, print message about success

    let github_client = GithubApi::new();
    let latest_release = github_client.get_latest_release().await?;

    let latest_version = parse_version(latest_release.tag_name.as_str())?;

    // if we want to force the update, set the version to 0.0.0 so its always out of date.
    // it's a hack.. but im lazy :3
    let current_version = if force_update {
        parse_version("0.0.0")?
    } else {
        parse_version(cargo_crate_version!())?
    };

    if latest_version == current_version && !force_update {
       notice(format!("Already up-to-date: {latest_version}"), Some(Color::Green), vec![Attribute::Bold]);
        return Ok(());
    }

    let archive_name = get_platform_bin_name()+".zip";

    let download_url = latest_release.assets.iter().find(|a| {
        a.name == archive_name
    }).map(|a| &a.browser_download_url);

    let Some(download_url) = download_url else {
        return Err(RustiqueError::SimpleError("Failed to get download url".into()));
    };

    let client = ApiClient::new();

    // create a unique path to work with our update
    let temp = temp_dir().join(Uuid::new_v4().to_string());
    if !temp.exists() {
        fs::create_dir_all(&temp).await?;
    }

    download_file(&client, download_url, temp.join(&archive_name), format!("Rustique archive saved to {}", temp.join(&archive_name).display())).await?;


    

    Ok(())
}


pub fn get_platform_bin_name() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64")     => "rustique-linux-x86_64".into(),
        ("linux", "aarch64")    => "rustique-linux-aarch64".into(),
        ("macos", "x86_64")     => "rustique-macos-x86_64".into(),
        ("macos", "aarch64")    => "rustique-macos-aarch64".into(),
        ("windows", "x86_64")   => "rustique-windows-x86_64".into(),
        _ => panic!("Unable to update binary, unsupported platform. Please open a github issue and state your platform.")
    }
}