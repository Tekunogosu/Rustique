pub trait StrLowerExt{
    fn lower_contains(&self, needle: &str) -> bool;
}

impl StrLowerExt for String {
    fn lower_contains(&self, needle: &str) -> bool {
        let needle_lower = needle.to_lowercase();
        self.to_lowercase().contains(&needle_lower)
    }
    
}


