
// Modpack creation is a bit involved with all the steps required. 
// using --interactive is the best way to do it so rustique can ask you questions as there are alot of flags by default

// only a few are required, so a minimal modpack can be created pretty easily

use std::collections::HashMap;
use std::path::Path;
use semver::Version;
use tracing::warn;
use crate::commands::arg_structs::modpack_args::MPCreateArgs;
use crate::commands::search::parse_search_file;
use crate::modpack::modpack_zip::{MPMods, ModPack, ModPackZip};
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_all_mods_metadata, find_mod_id};
use crate::version_management::parse_version;
use owo_colors::OwoColorize;
use crate::aliases::ModID;
use crate::config::config_manager::get_config;
use crate::consts::FILE_MODINFO_JSON;

pub fn mp_create_interactive() -> Result<(), RustiqueError> {
    todo!();
    Ok(())
}

pub fn collect_mp_create_args(args: &MPCreateArgs) -> Result<ModPackZip, RustiqueError> {

    Ok(ModPackZip {
        modpack: ModPack {
            name: args.name.clone(),
            mpk_id: args.mpk_id.clone(),
            version: args.mpk_version.clone(),
            game_version: args.game_version.clone(),
            description: args.description.clone(),
            author: args.author.clone(),
            contact: args.contact.clone(),
            website: args.website.clone(),
        },
        mods: HashMap::new(),
    })
}


pub async fn mp_create<P: AsRef<Path>>(mod_dir: P, mod_pack: &mut ModPackZip) -> Result<(), RustiqueError> {
    
    let config = get_config().read().await;
    
    let mods_search_data = parse_search_file().await?.mods;
    
    let all_mods = extract_all_mods_metadata(mod_dir, false).await?;
    let mp_mods: HashMap<ModID, MPMods> = all_mods.iter().filter_map(|(mod_filename, mod_info)| {
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
        
        Some((mod_info.name.clone(),MPMods {
            mod_id,
            version: version.to_string()
        }))
    }).collect();

    mod_pack.mods = mp_mods;
    
    // TODO: make flag for saving modpack to a different directory
    let save_location = Path::new(&config.modpacks.modpack_dir).to_path_buf();
    
    mod_pack.build_modpack(save_location, mod_pack.modpack.mpk_id.clone())?;


    Ok(())
}