use crate::aliases::ModID;
use crate::api::api_structs::ModInfo;
use crate::commands::sync::{get_sync_data, ModSyncInfo};
use crate::config_manager::get_config;
use crate::install_manager::Install;
use crate::rustique_errors::RustiqueError;
use crate::utils::{extract_all_mods_metadata, gather_dependencies, gather_missing_dependencies, sanitize_string};
use crate::version_management::parse_version;
use colored::Colorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Row, Table};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub async fn list_installed(mod_dir: &PathBuf, only_updated: bool) -> Result<(), RustiqueError> {
    let start_time = Instant::now();
    let config = get_config().read().unwrap();

    // check for sync data so we can show latest version
    let sync_data = get_sync_data(mod_dir).await?.rustique_sync;
    let mut table = setup_table_from_sync(Some(&sync_data));

    let installed_mods = extract_all_mods_metadata(&mod_dir)?;

    let missing_dependencies = gather_missing_dependencies(&installed_mods, &vec![], &sync_data)?;
    let all_dependencies = gather_dependencies(&installed_mods)?;

    let mut metadata: Vec<&ModInfo> = installed_mods.values().collect();

    let total_mod_count = metadata.len();

    metadata.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let metadata: Vec<&ModInfo> = if only_updated {
        metadata.into_iter().filter(|mod_info| {
            if let Some(sync) = sync_data.get(mod_info.mod_id.as_str()) {
                let latest = sync.latest_known_version.to_string();
                let current = sync.installed_version.to_string();
                return &latest != &current;
            } else {
                false
            }
        }).collect::<Vec<&ModInfo>>()
    } else {
        metadata
    };

    if metadata.is_empty() {
        eprintln!("{}","All mods are up to date!".green().bold());
        return Ok(());
    }


    metadata.into_iter().for_each(|mod_info| {
        let mut row = Row::new();
        row.add_cell(Cell::new(&mod_info.name).fg(Color::Yellow))
            .add_cell(Cell::new(&mod_info.mod_id));

        let installed_version = parse_version(mod_info.version.clone().unwrap_or_default()).unwrap().to_string();
        let installed_version_cell = Cell::new(&installed_version).add_attribute(Attribute::Dim);
        let mut latest_version_cell: Cell = Cell::new("N/A");
        if let Some(sync) = sync_data.get(mod_info.mod_id.as_str()) {

            let latest_version = sync.latest_known_version.to_string();

            latest_version_cell = Cell::new(&sync.latest_known_version.to_string());

            if latest_version.eq(&installed_version) {
                latest_version_cell = latest_version_cell.fg(Color::Green).add_attribute(Attribute::Dim);
            } else {
                latest_version_cell = latest_version_cell.add_attribute(Attribute::Bold).fg(Color::Red);
            }
        }
        row.add_cell(installed_version_cell);

        if sync_data.len() > 0 {
            row.add_cell(latest_version_cell);
        }



        let dep_list = grab_this_mod_deps(mod_info, all_dependencies.clone());

        let mut dep_cell = Cell::new(dep_list);
        dep_cell = dep_cell.set_delimiter(',');


        let missing_dep = grab_this_mod_deps(mod_info, missing_dependencies.clone());

        let mut missing_dep_cell = Cell::new(missing_dep);
        missing_dep_cell = missing_dep_cell.set_delimiter(',');

        row
            .add_cell(dep_cell)
            .add_cell(missing_dep_cell.fg(Color::Red).add_attribute(Attribute::SlowBlink).add_attribute(Attribute::Bold))
            .add_cell(Cell::new(sanitize_string(mod_info.description.as_ref().unwrap_or(&"".to_string())))
            );

        table.add_row(row);
    });

    println!("{}", table);
    print!("{} {}", "Total Mod Count:".bright_green().bold().on_black(), total_mod_count.to_string().bright_purple().on_black());

    if config.show_execution_time {
        let elapsed = format!("{:.2}", start_time.elapsed().as_secs_f64());
        println!(" - {}: {}{}","List operation took".bright_green().bold().on_black(), elapsed.bright_purple().on_black(), "s".bright_yellow().on_black());
    } else {
        println!();
    }

    Ok(())
}

fn grab_this_mod_deps(mod_info: &ModInfo, dep_list: Vec<Install>) -> String {
    dep_list.iter()
        .filter(|i| mod_info.dependencies.as_ref().map_or(false, |deps| deps.contains_key(&i.mod_id)))
        .map(|i| i.mod_id.clone()).collect::<Vec<ModID>>().join(",")
}


fn setup_table_from_sync(sync_data: Option<&HashMap<ModID, ModSyncInfo>>) -> Table {
    let mut table = Table::new();

    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);


    let mut header = Row::new();
    header
        .add_cell(Cell::new("Name").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("ModID").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("Version").add_attribute(Attribute::Bold).fg(Color::Blue));

    if sync_data.is_some() {
        header.add_cell(Cell::new("Latest Version").add_attribute(Attribute::Bold).fg(Color::Blue));
    }

    header
        .add_cell(Cell::new("Dependencies").fg(Color::Blue))
        .add_cell(Cell::new("Missing Dependencies").add_attribute(Attribute::Bold).fg(Color::Blue))
        .add_cell(Cell::new("Description").add_attribute(Attribute::Bold).fg(Color::Blue));

    table.add_row(header);

    table
}
