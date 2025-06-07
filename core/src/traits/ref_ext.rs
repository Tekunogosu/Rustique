use std::path::Path;

/// Trait for  AsRef\<Path\>
pub trait PathRef: AsRef<Path> {}
impl<T: AsRef<Path>> PathRef for T {}


pub trait StrRef: AsRef<str> {}
impl<S: AsRef<str>> StrRef for S {}
