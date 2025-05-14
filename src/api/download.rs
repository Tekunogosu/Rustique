use crate::api::client::ApiClient;
use crate::install_manager::{Install, Installed};
use crate::rustique_errors::RustiqueError;
use crate::utils::{verify_zip_file};
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tracing::{debug,info, warn};
use url::Url;
use crate::rustique_errors::RustiqueError::UrlParseError;

pub async fn download_requested_mods(mod_dir: &Path, mods_requested: &mut Vec<Install>, api_client: &ApiClient) -> Result<Vec<Installed>, RustiqueError> {

    let mut tasks = Vec::with_capacity(mods_requested.len());

    // create the outgoing vec with a capacity of what's being requested as we know it ahead of time


    while let Some(mod_request) = mods_requested.pop() {

        info!("{} {}", "Attempting to download mod".bright_green(), mod_request.mod_id.to_string().bright_yellow());

        let client = api_client.clone();
        let dir = mod_dir.to_path_buf();

        let task = tokio::spawn(async move {

            let mut installed = Installed {
                mod_id: mod_request.mod_id.clone(),
                mod_name: mod_request.mod_name.clone(),
                installed_file_path: None,
                old_file_path: mod_request.current_file_path.clone(),
                install_version:mod_request.version_to_install.clone(),
                success: false,
            };

            match download_mod(&dir, mod_request.download_url.clone(), &client).await {
                Ok(installed_path) => {

                    info!("{} {}: {}", "Successfully downloaded mod".bright_green(), mod_request.mod_id.magenta(), installed_path.display().to_string().bright_yellow());
                    installed.installed_file_path = Some(installed_path);
                    installed.success = true;
                    installed.clone()
                }
                Err(e) => {
                    warn!("Failed to download mod: {}, {}", mod_request.download_url, e);
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



async fn download_mod(mod_dir: &Path, download_url: String, api_client: &ApiClient) -> Result<PathBuf, RustiqueError> {
    let filename_from_api = &download_url.split('=').next_back().unwrap();

    // Replace any spaces in the downloaded file with _ . This makes it easier to process later
    let filename_fix = mod_dir.to_path_buf().join(filename_from_api).to_string_lossy().replace(' ', "_");
    let requested_file_path = PathBuf::from(filename_fix);


    let url = Url::parse(download_url.as_str())
        .map_err(UrlParseError)?;
    debug!("Trying to download url: {}", url.clone().to_string());

    // Retry logic - attempt download up to 3 times
    let max_retries = 3;
    let mut attempt = 0;
    let mut last_error = None;

    while attempt < max_retries {
        attempt += 1;
        info!("{}: [{}] {}","Download attempt".bright_blue(), attempt.to_string().magenta(), url.to_string().bright_yellow());

        match download_and_verify(&url, &requested_file_path, api_client).await {

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

                last_error = Some(e);

                // Add a small delay between retries
                if attempt < max_retries {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| RustiqueError::SimpleError("Maximum retries exceeded".to_string())))
}

async fn download_and_verify(url: &Url, file_path: &PathBuf, api_client: &ApiClient) -> Result<PathBuf, RustiqueError> {
    let response = api_client.get_request(url.as_ref()).await
        .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;

    // Check if we got a successful response
    if !response.status().is_success() {
        return Err(RustiqueError::SimpleError(
            format!("Server returned error status: {}", response.status())
        ));
    }

    // Get the full response body
    let bytes = response.bytes().await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failure reading response from API {url}"),
            source: std::io::Error::other(e),
        })?;

    // Verify we have actual content
    if bytes.is_empty() {
        return Err(RustiqueError::SimpleError("Downloaded file is empty".to_string()));
    }

    // Create and write to temp file first
    let temp_file_path = file_path.with_extension("tmp");

    let mut file = tokio::fs::File::create(&temp_file_path).await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Unable to create temp file {}", temp_file_path.to_string_lossy()),
            source: e
        })?;

    file.write_all(&bytes).await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failure while writing to file {}", temp_file_path.to_string_lossy()),
            source: e
        })?;

    // Ensure all data is written to disk
    file.sync_all().await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to flush file data for {}", temp_file_path.to_string_lossy()),
            source: e
        })?;

    // Close the file
    drop(file);

    // Pre-verify the zip file
    verify_zip_file(&temp_file_path)?;

    // Rename temp file to final file
    tokio::fs::rename(&temp_file_path, file_path).await
        .map_err(|e| RustiqueError::IoError {
            context: format!("Failed to rename temp file to {}", file_path.to_string_lossy()),
            source: e
        })?;

    debug!("File downloaded to {}", file_path.display());

    Ok(file_path.clone())
}

