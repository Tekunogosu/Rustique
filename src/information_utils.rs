use std::time::Instant;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Row, Table};
use comfy_table::ContentArrangement::Dynamic;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_BORDERS_ONLY, UTF8_FULL_CONDENSED, UTF8_HORIZONTAL_ONLY};
use crate::config::config_structs::{CellAttr, CellColor};
use crate::install_manager::Installed;
use crate::traits::ref_ext::StrRef;

pub struct RustiqueMessage {
    pub header: Option<CellData>,
    pub message: Vec<CellData>
}

pub fn rustique_message(rustique_message: RustiqueMessage) {
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY)
         .apply_modifier(UTF8_ROUND_CORNERS)
         .set_content_arrangement(Dynamic);

    if rustique_message.header.is_some() {
        let header_data = rustique_message.header;
        if header_data.is_some() {
            let header_data = header_data.unwrap_or_default();
            let mut h_cell = Cell::new(header_data.text);
            if !header_data.attributes.is_empty() {
                h_cell = h_cell.add_attributes(header_data.attributes);
            }
            h_cell = h_cell.fg(header_data.color.unwrap_or(Color::Green))
                           .set_alignment(header_data.alignment.unwrap_or(CellAlignment::Center));

            let mut row = Row::new();
            row.add_cell(h_cell);

            table.set_header(row);
        }
    }

    let rows: Vec<Row> = rustique_message.message.iter().map(|message_data|{
        let mut cell = Cell::from(message_data.text.clone());

        if !message_data.attributes.is_empty() {
            for attr in &message_data.attributes {
                cell = cell.add_attribute(*attr);
            }
        }
        cell = cell.fg(message_data.color.unwrap_or(Color::Yellow))
                   .set_alignment(message_data.alignment.unwrap_or(CellAlignment::Center));

        let mut row = Row::new();
        row.add_cell(cell);
        row
    }).collect();

    table.add_rows(rows);

    println!("{table}");
}

pub fn notice(message: impl StrRef, fg_color: Option<Color>, attributes: Vec<Attribute>) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_HORIZONTAL_ONLY)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(Dynamic);

    let mut cell = Cell::new(message.as_ref());

    if let Some(color) = fg_color {
        cell = cell.fg(color);
    }

    if !attributes.is_empty() {
        cell = cell.add_attributes(attributes);
    }

    cell = cell.set_alignment(CellAlignment::Center);

    let mut row = Row::new();
    row.add_cell(cell);

    table.add_row(row);
    println!("{table}");
}

pub fn prep_cell(text: impl StrRef, color: Option<CellColor>, attribute: Option<CellAttr>, delimiter: Option<char>, alignment: Option<CellAlignment>) -> Cell {
    let mut cell = Cell::from(text.as_ref());

    if color.is_some() {
        cell = cell.fg(Color::from(color.unwrap_or(CellColor::Reset)));
    }

    if attribute.is_some() {
        cell = cell.add_attribute(Attribute::from(attribute.unwrap_or(CellAttr::NoHidden)));
    }

    if delimiter.is_some() {
        cell = cell.set_delimiter(delimiter.unwrap_or(' '));
    }
    
    if alignment.is_some() {
        cell = cell.set_alignment(alignment.unwrap_or(CellAlignment::Left));
    }

    cell
}

fn fill_table_body(list: &mut [Installed], table: &mut Table, l_color: Color, r_color: Color) {
    list.sort_by(|a,b| a.mod_name.cmp(&b.mod_name));

    for m in list {
        let mut row = Row::new();
        row.add_cell(Cell::new(m.mod_name.clone()).fg(l_color).set_alignment(CellAlignment::Left));
        row.add_cell(Cell::new(m.install_version.clone()).fg(r_color).set_alignment(CellAlignment::Left));
        table.add_row(row);
    }
}

pub fn display_installation_results(mods_processed: Vec<Installed>) {
    let (mut successful, mut failed): (Vec<Installed>, Vec<Installed>) = mods_processed.into_iter().partition(|m| m.success);

    let mut s_table = Table::new();
    s_table.load_preset(UTF8_FULL_CONDENSED).apply_modifier(UTF8_ROUND_CORNERS);
    let mut f_table = s_table.clone();


    if !successful.is_empty() {
        let mut sh_row = Row::new();
        sh_row.add_cell(Cell::new("Successfully Installed".to_string()).fg(Color::Green).add_attribute(Attribute::Bold).set_alignment(CellAlignment::Center));
        s_table.set_header(sh_row);

        fill_table_body(&mut successful, &mut s_table, Color::Green, Color::Magenta);

        println!("{s_table}");

        display_table(vec![command_output("Total mods Installed".to_string(), successful.len().to_string())], None);
    }

    if !failed.is_empty() {
        let mut fh_row = Row::new();
        fh_row.add_cell(Cell::new("Failed to Install".to_string()).fg(Color::Red).add_attribute(Attribute::Bold).set_alignment(CellAlignment::Center));
        f_table.set_header(fh_row);

        fill_table_body(&mut failed, &mut f_table, Color::Red, Color::Magenta);

        println!("{f_table}");
    }
}

pub fn construct_cell(dt: CellData) -> Cell {
    let mut cell = Cell::new(dt.text);

    if let Some(color) = dt.color {
        cell = cell.fg(color);
    }

    for attr in dt.attributes {
        cell = cell.add_attribute(attr);
    }

    cell
}
pub fn command_output(option: impl StrRef, val: impl StrRef) -> (CellData, CellData) {
    (
        CellData::new(option.as_ref().into(), Some(Color::Yellow), vec![Attribute::Bold], None),
        CellData::new(val.as_ref().into(), Some(Color::Magenta), vec![Attribute::Bold], None),
    )
}

pub fn display_table(row_data: Vec<(CellData, CellData)>, table_style: Option<&str>) {
    let style = table_style.unwrap_or(UTF8_BORDERS_ONLY);
    let mut table = Table::new();
    table.load_preset(style).apply_modifier(UTF8_ROUND_CORNERS);

    let mut rows: Vec<Row> = Vec::new();

    for (l_col, r_col) in row_data {
        let mut row = Row::new();
        row.add_cell(construct_cell(l_col));
        row.add_cell(construct_cell(r_col));
        rows.push(row);
    }

    table.add_rows(rows);

    println!("{table}");
}

#[derive(Default)]
pub struct CellData {
    pub(crate) text: String,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) color: Option<Color>,
    pub(crate) alignment: Option<CellAlignment>
}

impl CellData {
    pub fn new(text: String, color: Option<Color>, attributes: Vec<Attribute>, alignment: Option<CellAlignment>) -> CellData {
        Self {
            text,
            attributes,
            color,
            alignment
        }
    }
}

pub fn elapsed_footer(start_time: Instant, operation: impl StrRef + std::fmt::Display) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_HORIZONTAL_ONLY)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(Dynamic);

    let elapsed = format!("{:.2}s", start_time.elapsed().as_secs_f64());
    // let out_str = format!("{} {} {}{}", operation.bright_green().bold(),"operation took:".bright_green().bold(), elapsed.bright_purple(), "s".bright_yellow());
    let operation_str = format!("{} {}", operation, "operation completed: ");
    let mut row = Row::new();

    row.add_cell(Cell::new(operation_str.as_str()).fg(Color::Green).add_attribute(Attribute::Bold));
    row.add_cell(Cell::new(elapsed.as_str()).fg(Color::Magenta).add_attribute(Attribute::Bold));

    table.add_row(row);

    println!("{table}");
}

