
pub type ModID = String;
pub type ModName = String;
pub type ModVersion = String;
pub type ModFileName = String;
pub type DownloadURL = String;
pub type FileName = String;
pub type UrlString = String;


pub type Tags = Vec<String>;
/// Used with the parse_{pinned,latest}_version functions
pub type PinnedVersionInfo = (ModVersion, DownloadURL, Tags, String);