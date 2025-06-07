use std::cmp::Ordering;
use std::str::FromStr;
use clap::ValueEnum;
use crate::api::api_structs::ModApi;
use crate::traits::option_ext::OptionExt;
use crate::traits::search_traits::{Searchable, Sortable};
use crate::traits::vec_ext::VecStringExt;

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

    fn matches_id(&self, id: i64) -> bool {
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


#[derive(Debug, PartialEq)]
pub enum SortValue {
    Number(i64),
    Text(String),
    Date(String),
}

impl PartialOrd for SortValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (SortValue::Number(a), SortValue::Number(b)) => a.partial_cmp(b),
            (SortValue::Text(a), SortValue::Text(b))
            | (SortValue::Date(a), SortValue::Date(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SearchCriteria {
    Text(String),
    Field {field: Field, value: String},
    Id(i64),
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
    pub fn add_id_search(mut self, id: i64) -> Self {
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