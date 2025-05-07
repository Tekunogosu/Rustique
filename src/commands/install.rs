use crate::aliases::{ModFileName, ModID, ModVersion};
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_all_mods_metadata, find_missing_dependencies, extract_zip_metadata, notice, elapsed_footer};
use colored::Colorize;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use comfy_table::{Attribute, Color};
use tracing::{debug, error, info, warn};
use crate::api::api_structs::ModInfo;
use crate::api::client::ApiClient;
use crate::commands::sync::{parse_json_file, ModSyncInfo, RustiqueSyncJson};
use crate::config_manager::get_config;
use crate::version_management::parse_latest_version;



pub async fn install_cmd(mod_dir: &PathBuf, mods_requested: Vec<ModID>, install_deps: bool) {

    // get sync data
    let sync_data = if let Ok(sync_data) = parse_json_file::<RustiqueSyncJson>() {
        sync_data
    } else {

    }


    // gather installed mods
    // if install_deps, gather missing deps


    // call install_manager




}
