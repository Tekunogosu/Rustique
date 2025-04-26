use std::fmt::format;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use crate::api::ApiClient;
use crate::sync::parse_sync_file;
use crate::utils::{dlog, RustiqueOptions};
use rayon::prelude::*;
use std::process::exit;
use url::{form_urlencoded, Url};

pub fn update(rustique_options: RustiqueOptions) -> Result<(), Box<dyn std::error::Error>> {
    let sync_data  = parse_sync_file(rustique_options.mod_dir.clone().unwrap());
    if sync_data.is_ok() {
        let sync_data = sync_data?;

        sync_data.rustique_sync.par_iter().for_each(|(mod_id, mod_sync_info)| {

            if mod_sync_info.latest_known_version != mod_sync_info.installed_version {
                let url = Url::parse(mod_sync_info.latest_download_url.as_str()).unwrap();
                dlog(format!("Trying to download url: {}", url.clone().to_string()).as_str());
                let response = ApiClient::new().download_mod(&url.to_string());

                match response {
                    Ok(result) => {
                        let mut bytes: Vec<u8> = Vec::new();
                        if let Ok(_) = result.into_body().into_reader().read_to_end(&mut bytes) {
                            let file_path = rustique_options.mod_dir.clone().unwrap().join(&mod_sync_info.latest_download_url.split('=').last().unwrap());
                            // create the file and write the bytes to it
                            let filename_fix = file_path.to_string_lossy().replace(" ", "_");
                            let path = Path::new(&filename_fix);
                            if let Ok(mut file) = File::create(path) {
                                if let Ok(_) = file.write_all(&bytes) {
                                    dlog(format!("File downloaded to {}", file_path.display()).as_str());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        dlog(format!("Something went wrong while downloading mod {}", mod_id.to_string()).as_str());
                        dlog(format!("{:?}", e).as_str());
                    }
                }
            }
        });

    } else {
        println!("Looks like you need to run './Rustique sync' first");
        exit(1);
    }


    Ok(())
}