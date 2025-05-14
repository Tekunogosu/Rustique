

// This trait to add helper functions to Vec<String> for ease of use in searching ignoring case

#[allow(unused_variables)]
pub trait VecStringExt {
    fn contains_ignore_case(&self, query: &str) -> bool;
    #[allow(unused)]
    fn contains_any(&self, queries: &[&str]) -> bool;
}

impl VecStringExt for Vec<String> {
    fn contains_ignore_case(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.iter().any(|s| s.to_lowercase().contains(&query_lower))
    }

    #[allow(unused_variables)]
    fn contains_any(&self, queries: &[&str]) -> bool {
        self.iter().any(|q| self.contains_ignore_case(q))
    }
}

