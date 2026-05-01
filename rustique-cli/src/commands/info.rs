use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{CellAlignment, Color, ContentArrangement, Row, Table};
use tracing::debug;
use rustique_core::api::api_structs::Release;
use rustique_core::api::client::ApiClient;
use crate::commands::arg_structs::info_args::ModInfoArgs;
use rustique_core::config::config_structs::{CellAttr, CellColor};
use rustique_core::information_utils::{notice, prep_cell};
use rustique_core::rustique_errors::RustiqueError;
use rustique_core::utils::html_parse;

pub async fn info(args: &ModInfoArgs) -> Result<(), RustiqueError> {

    let mods_vec = args.mod_id.clone();

    let client = ApiClient::new();
    let mod_info = client.fetch_mods_parallel(mods_vec).await?;

    debug!("{:?}", mod_info);
    
    for (_, mod_info) in mod_info {
       
        let mod_info = mod_info.mod_json;
        
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic);

        let mut t1 = table.clone();

        let headers = vec!{
            prep_cell(mod_info.mod_id.to_string(), Some(CellColor::Magenta), Some(CellAttr::Bold), None, None),
            prep_cell(mod_info.name.clone().unwrap_or_default(), Some(CellColor::Green), Some(CellAttr::Bold), None, Some(CellAlignment::Center)),
        };
        t1.set_header(Row::from(headers));

        let mut t1_rows: Vec<Row> = vec![];

        if let Some(author) = &mod_info.author {
           t1_rows.push(Row::from(vec![
                prep_cell("Author", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
                prep_cell(author, Some(CellColor::Green), None, None, None),
            ]));
        }

        t1_rows.push(
            Row::from(vec![
                prep_cell("URL", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
                prep_cell(format!("https://mods.vintagestory.at/show/mod/{}", &mod_info.asset_id), Some(CellColor::Yellow), None, None, None),
            ]),
        );

        if let Some(homepage) = &mod_info.home_page_url {
            t1_rows.push(Row::from(vec![
                prep_cell("Website", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
                prep_cell(homepage, Some(CellColor::Yellow), None, None, None),
            ]));
        }

        if let Some(source_code) = &mod_info.source_code_url {
            t1_rows.push(Row::from(vec![
                prep_cell("Source Code", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
                prep_cell(source_code, Some(CellColor::Yellow), None, None, None),
            ]));
        }

        if let Some(issue_tracker) = &mod_info.issue_tracker_url {
            t1_rows.push(Row::from(vec![
                prep_cell("Issue Tracker", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
                prep_cell(issue_tracker, Some(CellColor::Yellow), None, None, None),
            ]));
        }
        
        if let Some(wiki) = &mod_info.wiki_url {
            t1_rows.push(Row::from(vec![
                prep_cell("Wiki", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
                prep_cell(wiki, Some(CellColor::Yellow), None, None, None),
            ]));
        }

        if let Some(side) = &mod_info.side {
            t1_rows.push(Row::from(vec![
                prep_cell("Side", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
                prep_cell(side, Some(CellColor::Cyan), None, None, None),
            ]));
        }

        if !&mod_info.tags.is_empty() {
            t1_rows.push(Row::from(vec![
                prep_cell("Tags", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
                prep_cell(mod_info.tags.join(", "), Some(CellColor::Blue), None, Some(','), None),
            ]));
        }

        t1_rows.push(Row::from(vec![
            prep_cell("Downloads", Some(CellColor::Green), Some(CellAttr::Bold), None, None ),
            prep_cell(mod_info.downloads.to_string(), Some(CellColor::Magenta), None, None, None)
        ]));

        t1_rows.push(Row::from(vec![
            prep_cell("Follows", Some(CellColor::Green), Some(CellAttr::Bold), None, None ),
            prep_cell(mod_info.follows.to_string(), Some(CellColor::Magenta), None, None, None)
        ]));
        
        t1_rows.push(Row::from(vec![
            prep_cell("Comments", Some(CellColor::Green), Some(CellAttr::Bold), None, None ),
            prep_cell(mod_info.comments.to_string(), Some(CellColor::Magenta), None, None, None)
        ]));
        

        t1.add_rows(t1_rows);

        println!("{t1}");

        if args.show_description {
            let mut t2 = table.clone();
            // t2.load_preset(UTF8_HORIZONTAL_ONLY);
            let t2_txt = html_parse(&mut mod_info.text.clone().unwrap_or_default(), 100)?;
            t2.add_row(Row::from(vec![prep_cell(&t2_txt, Some(CellColor::Yellow), None, None, Some(CellAlignment::Left))]));

            println!("{t2}");
        }

        let mut versions_table = table.clone();
        versions_table.set_header(Row::from(vec![
            prep_cell("Version", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
            prep_cell("Game Versions", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
            prep_cell("Changelog", Some(CellColor::Green), Some(CellAttr::Bold), None, Some(CellAlignment::Center)),
        ]));

        let mut vt_rows: Vec<Row> = vec![];

        let rels = if args.show_versions == 0  {
            &mod_info.releases
        } else {
            &mod_info.releases.iter().take(args.show_versions).cloned().collect::<Vec<Release>>()
        };
       
        for (index, mv) in rels.iter().enumerate() {
            let version = &mv.mod_version.clone().unwrap_or_default().clone();
            let game_versions = &mv.tags.join(", ");
            let changelog = html2text::from_read(&mut mv.changelog.clone().unwrap_or(String::new()).as_bytes(), 100)
                .map_err(|_| RustiqueError::SimpleError("html2txt failed".to_string()))?;
            
            
            let cell_color = if index % 2 == 0 {
                CellColor::Green
            } else {
                CellColor::Cyan
            };
            
            vt_rows.push(Row::from(vec![
                prep_cell(version, Some(CellColor::Magenta), None, None, None),
                prep_cell(game_versions, Some(CellColor::Yellow), None, Some(','), None),
                prep_cell(&changelog, Some(cell_color), None, None, None),
            ]));
        }

        versions_table.add_rows(vt_rows);

        println!("{versions_table}");
        
        if rels.len() < mod_info.releases.len() {
            notice("By default, this list is limited to 3 versions. To see all versions, use [-v 0]. To see an arbitrary number of versions n, use [-v n].", Some(Color::Yellow), vec![]);
        }
        
    }
    


    Ok(())
}
