use std::cmp::Ordering;
use crate::commands::search::{Field, SortBy};

pub trait Searchable {
    fn matches_text(&self, query: &str) -> bool;
    fn matches_field(&self, field: &Field, value: &str) -> bool;
    fn matches_id(&self, id: u32) -> bool;
    fn matches_tag(&self, tag: &str) -> bool;
}

pub trait Sortable {
    fn get_sort_by(&self, field: &SortBy) -> SortValue;
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