use std::path::{Path, PathBuf};
use std::process::exit;
use comfy_table::{Attribute, Color};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::info;
use crate::api::client::{ApiClient, VSMirrorType};
use crate::commands::arg_structs::download_args::DownloadArgs;
use crate::config::config_manager::get_config;
use crate::information_utils::notice;
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::{PathRef, StrRef};
use crate::traits::string_ext::StrLowerExt;
use crate::utils::sorted_game_versions;

pub async fn download(args: &DownloadArgs) -> Result<(), RustiqueError> {
    args.validate()?;

    let config = get_config().read().await;
    let mut download_dir = match &args.save_dir.clone() {
        Some(dir) => dir.clone(),
        None => {
            config.game_download_dir.clone()
        }
    };

    if !Path::new(&download_dir).exists() {
        download_dir = String::new();
    }


    info!("Saving vintage story executable to: {}", &download_dir);

    let client = ApiClient::new();

    // if true, then use unstable, -rc.x is unstable
    let mirror_type = match &args.game_version.lower_contains("-rc") {
        true => VSMirrorType::Unstable,
        false => VSMirrorType::Stable
    };

    // verify game version
    let game_versions = sorted_game_versions().await;

    let user_version = args.game_version.replace('v', "");
    let mut found = false;
    for game_version in &game_versions {

        if game_version.replace('v', "").eq_ignore_ascii_case(&user_version) {
            found = true;
        }
    }

    if !found {
        notice(format!("The version you provided [{user_version}] is not valid. The following are all valid versions.."), Some(Color::Red), vec![Attribute::Bold]);
        notice(format!("[{}]",game_versions.join("], [").as_str()), Some(Color::Magenta), vec![]);
        exit(1);
    }


    notice(format!("Downloading Vintage Story v{user_version}"), Some(Color::Yellow), vec![Attribute::Bold]);

    let (url,filename) = client.download_uri(&args.os_type, &args.exe_type, &mirror_type, &user_version, Option::from(&args.windows_installer_type))?;

    info!("{url:?}");
    
    let save_loc = PathBuf::from(&download_dir).join(&filename);
    download_file(&client, &url, &save_loc, "").await?;
    
    
    notice(format!("Vintage Story has been saved to {download_dir}/{filename}"), Some(Color::Green), vec![Attribute::Bold]);
    
    Ok(())
}

pub async fn download_file(client: &ApiClient, url: &str, save_loc: impl PathRef, finish_message: impl StrRef) -> Result<(), RustiqueError> {
    let response = client.head(&url).await?;
    let total_size = response.headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse::<u64>().ok())
        .unwrap_or(0);
    
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise:.yellow}] [{bar:40.green/grey}] [{bytes:.cyan}/{total_bytes:.green}] [{percent:.magenta}%]")
            .unwrap().progress_chars("#}•")
    );
    
    let mut res = client.get_request(url).await?;
    let mut file = File::create(save_loc).await?;
    let mut downloaded = 0;

    while let Some(chunk) = res.chunk().await.map_err(|e| RustiqueError::SimpleError(e.to_string()))? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    } 
   
    pb.finish();

    if !finish_message.as_ref().is_empty() {
        notice(finish_message, Some(Color::Green), vec![Attribute::Bold]);
    }
    
    Ok(())
}