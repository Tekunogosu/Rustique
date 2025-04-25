use std::process::exit;
use crate::utils::RustiqueOptions;
use crate::sync::parse_sync_file;

pub fn update(rustique_options: RustiqueOptions) -> Result<(), Box<dyn std::error::Error>> {
    let sync_data  = parse_sync_file(rustique_options.mod_dir.unwrap());

    let mut mods_to_update: Vec<String> = Vec::new();

    if sync_data.is_ok() {
        let sync_data = sync_data?;
        sync_data.rustique_sync.iter().for_each(|(mod_id, mod_sync_info)| {
            if mod_sync_info.latest_known_version != mod_sync_info.installed_version {
                mods_to_update.push(mod_id.to_string());
            }
        });

        println!("Doing multithreaded download of updates...")
    } else {
        println!("Looks like you need to run './Rustique sync' first");
        exit(1);
    }


    Ok(())
}