//
// pub fn bulk_download(mod_dir: &PathBuf, num_mods_to_download: usize) -> Result<(), RustiqueError> {
//
//     let api = ApiClient::new();
//
//     let mod_download_urls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
//
//     let mut all_mods = api.fetch_all_mods().map_err(|err| RustiqueError::DownloadError(err.to_string()))?;
//
//     all_mods.mods.truncate(num_mods_to_download);
//
//     // iterate through the mods and make yet another api call to the actual mod and try and get a mod
//     // that is slightly out of date for testing
//
//     all_mods.mods.par_iter().for_each(|a_mod| {
//         println!("Checking {}", a_mod.mod_id);
//
//         if let Ok(mod_) = api.fetch_mod(a_mod.mod_id.to_string().as_str()) {
//             if mod_.mod_json.releases.len() > 1 {
//                mod_download_urls.lock().unwrap().push(mod_.mod_json.releases[1].main_file.clone().unwrap_or_default())
//             }
//         };
//     });
//
//     println!("found {} mods total", mod_download_urls.lock().unwrap().len());
//
//     mod_download_urls.lock().unwrap().par_iter().for_each(|download_url| {
//         // download and install the mod
//         match install_mod(&mod_dir, &download_url, &api) {
//             Ok(_) => {}
//             Err(e) => {
//                 println!("Failed to install mod: {}", e);
//             }
//         }
//     });
//
//     Ok(())
// }