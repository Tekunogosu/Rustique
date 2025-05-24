
use std::cmp::Ordering;
use std::str::FromStr;
use clap::ValueEnum;
use owo_colors::OwoColorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Row, Table};
use tracing::{debug, warn};
use crate::api::api_structs::{ModApi, ModsSearchFile};
use crate::commands::arg_structs::search_args::SearchArgs;
use crate::commands::sync::daily_file_syncs;
use crate::config::config_manager::{get_config, Config};
use crate::config::config_structs::SearchColumn;
use crate::consts::FILE_MOD_SEARCH_SYNC;
use crate::information_utils::prep_cell;
use crate::rustique_errors::RustiqueError;
use crate::traits::option_ext::OptionExt;
use crate::traits::search_traits::{Searchable, SortValue, Sortable};
use crate::traits::vec_ext::VecStringExt;
use crate::utils::{parse_json_file};




impl Searchable for ModApi {
    fn matches_text(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.name.matches_contains(&query)
        || self.summary.matches_contains(&query)
        || self.author.matches_contains(&query)
        || self.mod_type.matches_contains(&query)
        || self.side.matches_contains(&query)
        || self.mod_id_strs.contains(&query)
        || self.url_alias.matches_contains(&query)
        || self.tags.contains(&query)
    }

    #[allow(clippy::match_wildcard_for_single_variants)]
    fn matches_field(&self, field: &Field, value: &str) -> bool {
        match field {
            Field::Name     => self.name.matches_contains(value),
            Field::Summary  => self.summary.matches_contains(value),
            Field::Author   => self.author.matches_contains(value),
            Field::ModType  => self.mod_type.matches_contains(value),
            Field::Side     => self.side.matches_contains(value),
            Field::ModIdStr => self.mod_id_strs.contains(&value.to_string()),
            Field::UrlAlias => self.url_alias.matches_contains(value),
            _ => false
        }
    }

    fn matches_id(&self, id: u32) -> bool {
        self.mod_id == id || self.asset_id == id
    }

    fn matches_tag(&self, tag: &str) -> bool {
        self.tags.contains_ignore_case(&tag.to_lowercase())
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Field {
    Name,
    Summary,
    Author,
    ModType,
    Side,
    ModIdStr,
    UrlAlias,
    Tags
}

#[derive(ValueEnum, Clone, Debug)]
pub enum SortBy {
    ModId,
    AssetId,
    Downloads,
    Follows,
    Trending,
    Comments,
    Name,
    Author,
    Released,
}

impl Sortable for ModApi {
      fn get_sort_by(&self, field: &SortBy) -> SortValue {
        match *field {
            SortBy::Name        => SortValue::Number(i64::from(self.mod_id)),
            SortBy::AssetId     => SortValue::Number(i64::from(self.asset_id)),
            SortBy::Downloads   => SortValue::Number(i64::from(self.downloads)),
            SortBy::Follows     => SortValue::Number(i64::from(self.follows)),
            SortBy::Author      => SortValue::Number(i64::from(self.trending_points)),
            SortBy::Released    => SortValue::Number(i64::from(self.comments)),
            SortBy::Comments    => SortValue::Text(self.name.clone().unwrap_or_default()),
            SortBy::Trending    => SortValue::Text(self.author.clone().unwrap_or_default()),
            SortBy::ModId       => SortValue::Date(self.last_released.clone().unwrap_or_default()),
        }
    }
}



#[derive(Debug, Clone)]
pub enum SearchCriteria {
    Text(String),
    Field {field: Field, value: String},
    Id(u32),
    Tag(String),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortOrder {
    Asc,
    Desc
}

impl FromStr for SortOrder {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "asc" | "ascending" => Ok(SortOrder::Asc),
            "desc" | "descending" => Ok(SortOrder::Desc),
            _ => Err(format!("Invalid sort order: {s}"))
        }
    }
}

#[allow(dead_code, unused)]
#[derive(Debug)]
pub struct SearchQuery {
    pub criteria: Vec<SearchCriteria>,
    pub sort_by: Option<SortBy>,
    pub sort_order: SortOrder,
}

#[allow(unused)]
impl SearchQuery {
    pub fn new() -> Self {
        SearchQuery {
            criteria: Vec::new(),
            sort_by: None,
            sort_order: SortOrder::Asc,
        }
    }

    pub fn add_text_search(mut self, text: String) -> Self {
        self.criteria.push(SearchCriteria::Text(text));
        self
    }

    pub fn add_field_search(mut self, field: Field, value: String) -> Self {
        self.criteria.push(SearchCriteria::Field {field, value});
        self
    }

    #[allow(dead_code)]
    pub fn add_id_search(mut self, id: u32) -> Self {
        self.criteria.push(SearchCriteria::Id(id));
        self
    }

    pub fn add_tag_search(mut self, tag: String) -> Self {
        self.criteria.push(SearchCriteria::Tag(tag));
        self
    }

    pub fn sort_by(mut self, field: SortBy) -> Self {
        self.sort_by = Some(field);
        self
    }

    pub fn with_sort(mut self, field: SortBy, order: SortOrder) -> Self {
        self.sort_by = Some(field);
        self.sort_order = order;
        self
    }

    pub fn execute(&self, mods: &[ModApi]) -> Vec<ModApi> {
        let mut results: Vec<ModApi> = mods
            .iter()
            .filter(|mod_item| {
                if self.criteria.is_empty() {
                    return true;
                }

                self.criteria.iter().all(|criterion| match criterion {
                    SearchCriteria::Text(query) => mod_item.matches_text(query),
                    SearchCriteria::Field{field, value} => mod_item.matches_field(field, value),
                    SearchCriteria::Id(id) => mod_item.matches_id(*id),
                    SearchCriteria::Tag(tag) => mod_item.matches_tag(tag),
                })
            }).cloned().collect();

        if let Some(sort_field) = &self.sort_by {
            results.sort_by(|a, b| {
                let a_val = a.get_sort_by(sort_field);
                let b_val = b.get_sort_by(sort_field);

                let order = a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal);

                match self.sort_order {
                    SortOrder::Asc => order,
                    SortOrder::Desc => order.reverse(),
                }
            });
        }

        results
    }
}

pub async fn parse_search_file() -> Result<ModsSearchFile, RustiqueError> {
    let file_path = Config::get_path().join(FILE_MOD_SEARCH_SYNC);
    if !file_path.exists() {
        warn!("{}","Running daily sync to build search table, this is normal!".green());
        daily_file_syncs(true).await?;
    }
    parse_json_file::<ModsSearchFile>(&file_path)
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

