
use core::search::SortBy;
use std::str::FromStr;
use owo_colors::OwoColorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Row, Table};
use tracing::{debug, warn};
use core::api::api_structs::{ModApi, ModsSearchFile};
use crate::commands::arg_structs::search_args::SearchArgs;
use crate::commands::sync::daily_file_syncs;
use core::config::config_manager::{get_config, Config};
use core::config::config_structs::SearchColumn;
use core::consts::FILE_MOD_SEARCH_SYNC;
use core::information_utils::prep_cell;
use core::rustique_errors::RustiqueError;
use core::utils::{parse_json_file};
use core::search::{Field, SearchQuery, SortOrder};




pub async fn parse_search_file() -> Result<ModsSearchFile, RustiqueError> {
    let file_path = Config::get_path().join(FILE_MOD_SEARCH_SYNC);
    if !file_path.exists() {
        warn!("{}","Running daily sync to build search table, this is normal!".green());
        daily_file_syncs(true).await?;
    }
    parse_json_file::<ModsSearchFile>(&file_path).await
}

pub async fn search(args: &SearchArgs) -> Result<(), RustiqueError> {

    let search_file = parse_search_file().await?;

    let mut query = SearchQuery::new();

    if args.field.is_some() && args.query.is_some() {
        query = query.add_field_search(args.field.clone().unwrap_or(Field::Summary), args.query.clone().unwrap_or_default());
    } else if args.query.is_some() {
        query = query.add_text_search(args.query.clone().unwrap_or_default());
    }


    if args.author.is_some() {
        query = query.add_field_search(Field::Author, args.author.clone().unwrap_or_default());
    }

    if args.tag.is_some() {
        query = query.add_tag_search(args.tag.clone().unwrap_or_default());
    }


    if args.sort_direction.is_some() {
        query.sort_order = args.sort_direction.unwrap_or(SortOrder::Asc);
    }

    if args.sort_by.is_some() {
        query.sort_by = Some(args.sort_by.clone().unwrap_or(SortBy::Name));
    }

    let res = query.execute(&search_file.mods);

    debug!("search result: {:#?}", res);

    show_search_table(res).await;


    Ok(())
}

pub async fn show_search_table(results: Vec<ModApi>) {
    let config = get_config().read().await;

    let search_config = &config.table.search;
    let search_headers = &search_config.headers;
    let search_cells = &search_config.cells;
    
    debug!("search headers: {search_headers:#?}");
    debug!("search cells: {search_cells:#?}");

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let col_cells: Vec<Cell> = search_headers.iter().map(|(k, v)| {
        let color = v.color.clone();
        let attr = v.attribute.clone();

        let col_txt = match <SearchColumn as FromStr>::from_str(k) {
            Ok(SearchColumn::Name)          => "Name",
            Ok(SearchColumn::Author)        => "Author",
            Ok(SearchColumn::ModId)         => "ModID",
            Ok(SearchColumn::ModidStrs)     => "ModID Strings",
            Ok(SearchColumn::AssetId)       => "AssetID",
            Ok(SearchColumn::Downloads)     => "Downloads",
            Ok(SearchColumn::Follows)       => "Follows",
            Ok(SearchColumn::Trending)      => "Trending Points",
            Ok(SearchColumn::Comments)      => "Comments",
            Ok(SearchColumn::Summary)       => "Summary",
            Ok(SearchColumn::UrlAlias)      => "Url Alias",
            Ok(SearchColumn::Side)          => "Side",
            Ok(SearchColumn::Type)          => "Type",
            Ok(SearchColumn::Tags)          => "Tags",
            Ok(SearchColumn::LastReleased)  => "Last Released",
            _ => "N/A"
        };

        prep_cell(col_txt, color, attr, None, None)
    }).collect();

    table.set_header(Row::from(col_cells));

    let b_rows: Vec<Row> = results.iter().map(|m| {
        let cells: Vec<Cell> = search_cells.iter().map(|(k,v)| {
            let color = v.color.clone();
            let attr = v.attribute.clone();

            let mut right_align = false;
            
            let col_txt = match <SearchColumn as FromStr>::from_str(k) {
                Ok(SearchColumn::Name)      => m.name.clone().unwrap_or_default(),
                Ok(SearchColumn::ModId)     => {
                    right_align = true;
                    m.mod_id.to_string()
                },
                Ok(SearchColumn::AssetId)   => {
                    right_align = true;
                    m.asset_id.to_string()
                },
                Ok(SearchColumn::Downloads) => {
                    right_align = true;
                    m.downloads.to_string()
                },
                Ok(SearchColumn::Follows)   => {
                    right_align = true;
                    m.follows.to_string()
                },
                Ok(SearchColumn::Trending)  => {
                    right_align = true;
                    m.trending_points.to_string()
                },
                Ok(SearchColumn::Comments)  => m.comments.to_string(),
                Ok(SearchColumn::Summary)   => m.summary.clone().unwrap_or_default(),
                Ok(SearchColumn::ModidStrs) => m.mod_id_strs.join(","),
                Ok(SearchColumn::Author)    => m.author.clone().unwrap_or_default(),
                Ok(SearchColumn::UrlAlias)  => m.url_alias.clone().unwrap_or_default(),
                Ok(SearchColumn::Side)      => m.side.clone().unwrap_or_default(),
                Ok(SearchColumn::Type)      => m.mod_type.clone().unwrap_or_default(),
                Ok(SearchColumn::Tags)      => m.tags.join(","),
                Ok(SearchColumn::LastReleased) => {
                    right_align = true;
                    m.last_released.clone().unwrap_or_default()
                },
                _ => String::new()
            };
            
            if right_align {
                prep_cell(&col_txt, color, attr, None, Some(CellAlignment::Right))
            } else {
                prep_cell(&col_txt, color, attr, None, None)
            }

        }).collect();

        Row::from(cells)
    }).collect();

    table.add_rows(b_rows);

    println!("{table}");
}

