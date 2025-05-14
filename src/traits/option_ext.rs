
use crate::traits::string_ext::StrLowerExt;

#[allow(dead_code)]
pub trait OptionExt {
    type Inner;

    fn matches_contains(&self, query: &str) -> bool;
    fn as_str_option(&self) -> Option<&str>;
    fn as_u32_option(&self) -> Option<u32>;
}

impl OptionExt for Option<String> {
    type Inner = String;
    fn matches_contains(&self, query: &str) -> bool {
        self.as_ref()
            .is_some_and(|s| s.lower_contains(&query.to_lowercase()))
    }

    fn as_str_option(&self) -> Option<&str> {
        self.as_deref()
    }

    fn as_u32_option(&self) -> Option<u32> {
        None
    }
}