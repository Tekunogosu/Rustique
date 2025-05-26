
// Modpack creation is a bit involved with all the steps required. 
// using --interactive is the best way to do it so rustique can ask you questions as there are alot of flags by default

// only a few are required, so a minimal modpack can be created pretty easily

use std::collections::HashMap;
use std::path::Path;
use semver::Version;
use tracing::warn;
use crate::commands::arg_structs::modpack_args::MPCreateArgs;
use crate::commands::search::parse_search_file;
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_all_mods_metadata, find_mod_id};
use crate::version_management::parse_version;
use owo_colors::OwoColorize;
use crate::aliases::{ModID, ModVersion};
use crate::api::api_structs::{ModInfo, StringOrInt};
use crate::config::config_manager::get_config;
use crate::consts::FILE_MODINFO_JSON;
use crate::traits::ref_ext::PathRef;
// pub fn mp_create_interactive() -> Result<(), RustiqueError> {
//     todo!();
//     // Ok(())
// }

pub fn collect_mp_create_args(args: &MPCreateArgs) -> Result<ModInfo, RustiqueError> {
    Ok(ModInfo {
        name: args.name.clone(),
        mod_type: StringOrInt::default(),
        mod_id: args.mpk_id.clone(),
        version: Some(args.mpk_version.clone()),
        network_version: None,
        texture_size: None,
        description: args.description.clone(),
        website: args.website.clone(),
        authors: vec![args.author.clone().unwrap_or_default()],
        contributors: vec![],
        side: None,
        required_on_client: None,
        required_on_server: None,
        dependencies: HashMap::default(),
    })
}


pub async fn mp_create<P: AsRef<Path>>(mod_dir: P, mod_pack: &mut ModInfo, save_location: Option<impl PathRef>) -> Result<(), RustiqueError> {
    
    let config = get_config().read().await;
    
    let mods_search_data = parse_search_file().await?.mods;
    
    let all_mods = extract_all_mods_metadata(mod_dir, false).await?;
    let mp_mods: HashMap<ModID, ModVersion> = all_mods.iter().filter_map(|(mod_filename, mod_info)| {
        let mod_id = if mod_info.mod_id.is_empty() {
            find_mod_id(&mod_info.name, mod_filename, &mods_search_data).unwrap_or_default()
        } else {
            mod_info.mod_id.clone()
        };
        
        if mod_id.is_empty() {
            warn!("{} {} {} {} {}","Mod".yellow(), mod_filename.magenta(), 
                "was not included in this modpack because Rustique was unable to locate a valid modid. It was either omitted or the mod has a malformed".yellow(), FILE_MODINFO_JSON.magenta(), "file".yellow());
            return None;
        }
        
        let version = parse_version(&mod_info.version.clone().unwrap_or("0.0.0".into()))
            .unwrap_or(Version::new(0,0,0));
        
        Some((mod_id, version.to_string()))
    }).collect();

    mod_pack.dependencies = mp_mods;
    
    // TODO: make flag for saving modpack to a different directory
    let save_location = if let Some(save_path) = save_location {
        Path::new(save_path.as_ref()).to_path_buf()
    } else {
        Path::new(&config.modpacks.modpack_dir).join("mypacks")
    };
    
    mod_pack.build_modpack(save_location, mod_pack.mod_id.clone())?;
    
    Ok(())
}