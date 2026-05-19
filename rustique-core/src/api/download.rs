use crate::api::client::ApiClient;
use crate::install_manager::{Install, Installed};
use crate::rustique_errors::RustiqueError;
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::io::AsyncWriteExt;
use tracing::{debug,info, warn};
use url::Url;
use crate::rustique_errors::RustiqueError::UrlParseError;
use crate::traits::ref_ext::PathRef;


pub async fn download_requested_mods(mod_dir: &Path, mods_requested: &mut Vec<Install>, api_client: &ApiClient, mp: Option<&MultiProgress>) -> Result<Vec<Installed>, RustiqueError> {

    let mut tasks = Vec::with_capacity(mods_requested.len());

    while let Some(mod_request) = mods_requested.pop() {
        info!("{} {}", "Attempting to download mod".bright_green(), mod_request.mod_id.to_string().bright_yellow());

        let client = api_client.clone();
        let dir = mod_dir.to_path_buf();

        let bar = mp.map(|mp| {
            let pb = mp.add(ProgressBar::new(0));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise:.cyan}] [{bar:.cyan/grey:40}] {wide_msg:.yellow} {bytes:.green}/{total_bytes:.cyan}")
                    .unwrap()
                    .progress_chars("█▒░")
            );
            let msg = format!("({}) : {}", mod_request.version_to_install, mod_request.mod_name.clone());
            pb.set_message(msg);
            pb
        });

        let task = tokio::spawn(async move {

            let mut installed = Installed {
                mod_id: mod_request.mod_id.clone(),
                mod_name: mod_request.mod_name.clone(),
                installed_file_path: None,
                old_file_path: mod_request.current_file_path.clone(),
                install_version: mod_request.version_to_install.clone(),
                success: false,
            };

            match download_mod(&dir, mod_request.download_url.clone(), &client, bar.as_ref()).await {
                Ok(installed_path) => {
                    info!("{} {}: {}", "Successfully downloaded mod".bright_green(), mod_request.mod_id.magenta(), installed_path.display().to_string().bright_yellow());
                    if let Some(pb) = bar {
                        let msg = format!("({}) : {}", mod_request.version_to_install, mod_request.mod_name.bright_green());
                        pb.finish_with_message(msg);
                    }
                    installed.installed_file_path = Some(installed_path);
                    installed.success = true;
                    installed.clone()
                }
                Err(e) => {
                    warn!("Failed to download mod: {}, {}", mod_request.download_url, e);
                    if let Some(pb) = bar {
                        let msg = format!("({}) : {}", mod_request.version_to_install, mod_request.mod_name.bright_red());
                        pb.finish_with_message(msg);
                    }
                    installed.clone()
                }
            }
        });

        tasks.push(task);
    }
    
    

    let mut result: Vec<Installed> = Vec::with_capacity(tasks.len());
    for task in tasks {
        match task.await {
            Ok(installed) => result.push(installed),
            Err(e) => warn!("Task join error in download_requested_mods: {}", e),
        }
    }

    Ok(result)
}



pub async fn download_mod(mod_dir: &Path, download_url: String, api_client: &ApiClient, pb: Option<&ProgressBar>) -> Result<PathBuf, RustiqueError> {
    let filename_from_api = &download_url.split('=').next_back().unwrap_or_default().replace(' ', "_");

    // Replace any spaces in the downloaded file with _ . This makes it easier to process later
    let filename_fix = mod_dir.to_path_buf().join(filename_from_api).to_string_lossy().to_string();
    let requested_file_path = PathBuf::from(filename_fix);

    let url = Url::parse(download_url.as_str())
        .map_err(UrlParseError)?;
    debug!("Trying to download url: {}", url.clone().to_string());

    // Retry logic - attempt download up to 3 times
    let max_retries = 3;
    let mut attempt = 0;

    while attempt < max_retries {
        attempt += 1;
        info!("{}: [{}] {}","Download attempt".bright_blue(), attempt.to_string().magenta(), url.to_string().bright_yellow());

        if let Some(pb) = pb {
            pb.set_position(0);
            pb.set_length(0);
        }

        match download_and_verify(&url, &requested_file_path, api_client, pb).await {

            // file_path here is the verified path after the file has been downloaded
            Ok(file_path) => {
                info!("{} {} {} {}","Successfully downloaded".bright_green(), file_path.display().to_string().bright_yellow(), "on attempt".bright_green(), attempt.to_string().magenta());
                return Ok(file_path);
            },
            Err(e) => {
                warn!("Download attempt {} failed for {}: {}", attempt, url, e);

                // Clean up any partial downloads
                if requested_file_path.exists() {
                    if let Err(clean_err) = tokio::fs::remove_file(&requested_file_path).await {
                        warn!("Failed to clean up partial download {}: {}", requested_file_path.display(), clean_err);
                    }
                }

                info!("{} {} {}", "Download failed on attempt ".yellow(), attempt.to_string().magenta(), e.to_string().red());

                // Add a small delay between retries
                if attempt < max_retries {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    Err(RustiqueError::SimpleError("Maximum retries exceeded".to_string()))
}

pub async fn download_and_verify(url: &Url, file_path: impl PathRef, api_client: &ApiClient, pb: Option<&ProgressBar>) -> Result<PathBuf, RustiqueError> {
    let file_path = file_path.as_ref();
    let response = api_client.get_request(url.as_ref()).await
        .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;

    // Check if we got a successful response
    if !response.status().is_success() {
        return Err(RustiqueError::SimpleError(
            format!("Server returned error status: {}", response.status())
        ));
    }

    if let (Some(pb), Some(len)) = (pb, response.content_length()) {
        pb.set_length(len);
    }

    // Create and write to temp file first, streaming chunks as they arrive
    let temp_file_path = file_path.with_extension("tmp");

    let mut file = tokio::fs::File::create(&temp_file_path).await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Unable to create temp file {}", temp_file_path.to_string_lossy()),
            source: e
        })?;

    let mut stream = response.bytes_stream();
    let mut total_bytes = 0u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| RustiqueError::IoError {
            context: format!("Failure reading response stream from {url}"),
            source: std::io::Error::other(e),
        })?;
        file.write_all(&chunk).await
            .map_err(|e| RustiqueError::IoError {
                context: format!("Failure while writing to file {}", temp_file_path.to_string_lossy()),
                source: e
            })?;
        total_bytes += chunk.len() as u64;
        if let Some(pb) = pb {
            pb.inc(chunk.len() as u64);
        }
    }

    if total_bytes == 0 {
        return Err(RustiqueError::SimpleError("Downloaded file is empty".to_string()));
    }

    // Ensure all data is written to disk
    file.sync_all().await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to flush file data for {}", temp_file_path.to_string_lossy()),
            source: e
        })?;

    drop(file);

    // Pre-verify the zip file
    // verify_zip_file(&temp_file_path).await?;

    // Rename temp file to final file
    tokio::fs::rename(&temp_file_path, file_path).await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to rename temp file to {}", file_path.to_string_lossy()),
            source: e
        })?;

    debug!("File downloaded to {}", file_path.display());

    Ok(file_path.into())
}

