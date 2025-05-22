
// Modpack creation is a bit involved with all the steps required. 
// using --interactive is the best way to do it so rustique can ask you questions as there are alot of flags by default

// only a few are required, so a minimal modpack can be created pretty easily

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use semver::Version;
use tracing::{debug, info, warn};
use crate::commands::arg_structs::modpack_args::MPCreateArgs;
use crate::commands::search::parse_search_file;
use crate::modpack::modpack_toml::{MPMods, ModPack, ModPackToml};
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_all_mods_metadata, find_mod_id};
use crate::version_management::parse_version;
use owo_colors::OwoColorize;
use serde_json::json;
use crate::aliases::ModID;
use crate::config::config_manager::{get_config, Config};

pub fn mp_create_interactive() -> Result<(), RustiqueError> {

    Ok(())
}

pub fn collect_mp_create_args(args: &MPCreateArgs) -> Result<ModPackToml, RustiqueError> {

    Ok(ModPackToml {
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


pub async fn mp_create(mod_dir: &PathBuf, mod_pack: &mut ModPackToml) -> Result<(), RustiqueError> {

   
    // parse all mods in mod_dir
    // grab each mod id and version
    // populate the mods section of the ModPackToml
    // write toml
    
    let config = get_config().read().await;
    
    let mods_search_data = parse_search_file()?.mods;
    
    let all_mods = extract_all_mods_metadata(mod_dir).await?;
    let mp_mods: HashMap<ModID, MPMods> = all_mods.iter().filter_map(|(mod_filename, mod_info)| {
        let mod_id = if mod_info.mod_id.is_empty() {
            find_mod_id(&mod_info.name, mod_filename, &mods_search_data).unwrap_or_default()
        } else {
            mod_info.mod_id.clone()
        };
        
        if mod_id.is_empty() {
            warn!("{} {} {}","Mod".yellow(), mod_filename.magenta(), "was not included in this modpack because Rustique was unable to locate a valid modid. It was either omitted or the mod has a malformed modinfo.json file".yellow());
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
    
    // write the mod_pack to the file
    
    debug!("{mod_pack:#?}");
    
   
    // TODO: make flag for saving modpack to a different directory
    let save_location = Config::get_path().join(Path::new(&format!("{}.toml", mod_pack.modpack.mpk_id)));
    
    
    mod_pack.save(&save_location)?;
    
    mod_pack.gen_modinfo_json(&Config::get_path())?;


    Ok(())
}