use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_FULL_CONDENSED, UTF8_HORIZONTAL_ONLY};
use comfy_table::{Cell, CellAlignment, ContentArrangement, Row, Table};
use crate::api::client::ApiClient;
use crate::commands::arg_structs::info_args::ModInfoArgs;
use crate::config_structs::{CellAttr, CellColor};
use crate::information_utils::prep_cell;
use crate::rustique_errors::RustiqueError;

pub async fn info(args: &ModInfoArgs) -> Result<(), RustiqueError> {

    let mod_id = args.mod_id.clone();

    let client = ApiClient::new();
    let mod_info = client.fetch_mod(&mod_id).await?.mod_json;

    let releases = &mod_info.releases;


    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let mut t1 = table.clone();

    let headers = vec!{
        prep_cell("ModID", Some(CellColor::Green), Some(CellAttr::Bold), None, None),
        prep_cell("Name", Some(CellColor::Green), None, None, None),
        prep_cell("Latest Version", Some(CellColor::Green), None, None, None),
    };
    t1.set_header(Row::from(headers));

    let t1_body = vec![
        prep_cell(&mod_info.mod_id.to_string(), None, None, None, Some(CellAlignment::Right)),
        prep_cell(&mod_info.name.clone().unwrap_or_default(), None, None, None, None),
        prep_cell(&releases.clone().first().unwrap().mod_version.clone().unwrap_or_default(), None, None, None, Some(CellAlignment::Right)),
    ];

    t1.add_row(Row::from(t1_body));

    println!("{t1}");

    let mut t2 = table.clone();
    t2.load_preset(UTF8_HORIZONTAL_ONLY);
    let t2_txt = html2text::from_read_rich(&mut mod_info.text.clone().unwrap_or_default().as_bytes(), 100).map_err(|e| RustiqueError::SimpleError("html2txt failed".to_string()))?;
    t2.add_row(Row::from(vec![prep_cell(&"".to_string(), Some(CellColor::Yellow), None, None, Some(CellAlignment::Center))]));

    println!("{t2}");


    Ok(())
}