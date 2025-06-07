use std::cmp::Ordering;
use crate::search::{Field, SortBy, SortValue};

pub trait Searchable {
    fn matches_text(&self, query: &str) -> bool;
    fn matches_field(&self, field: &Field, value: &str) -> bool;
    fn matches_id(&self, id: i64) -> bool;
    fn matches_tag(&self, tag: &str) -> bool;
}

pub trait Sortable {
    fn get_sort_by(&self, field: &SortBy) -> SortValue;
}

