use owo_colors::OwoColorize;
use std::fmt;
use crate::consts::FILE_MODINFO_JSON;

#[allow(dead_code)]
#[derive(Debug)]
pub enum RustiqueError {
    ApiError {
        context: String,
        source: reqwest::Error,
    },
    DownloadError(String),
    IoError{
        context: String,
        source: std::io::Error
    },
    UrlParseError(url::ParseError),
    VersionError{
        context: String,
        source: semver::Error,
    },
    NoVersionFound(String),
    JsonError {
        context: String,
        source: serde_json5::Error
    },
    SimpleError(String),
    ModNotZipped(String),
    ZipError {
        context: String,
        source: zip::result::ZipError
    },
    ConfigFileError(String),
    MalformedModInfoJson(String),
    TomlError{
        context: String,
        source: toml::de::Error
    },

}

impl fmt::Display for RustiqueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RustiqueError::ApiError {context, source} => write!(f, "Api Error: {}: {}", context, source.to_string().red().bold()),
            RustiqueError::DownloadError(e) => write!(f, "Download Error: {}", e.to_string().red().bold()),
            RustiqueError::IoError { context, source } => write!(f, "{}: {}", context, source.to_string().red().bold()),
            RustiqueError::UrlParseError(e) => write!(f, "Parse Error: {}", e.to_string().red().bold()),
            RustiqueError::SimpleError(e) => write!(f, "{}", e.to_string().red().bold()),
            RustiqueError::ZipError{context, source} => write!(f, "ZipError: {}, {}", context, source.to_string().red().bold()),
            RustiqueError::JsonError{context, source} => write!(f, "JsonParseError: {}, {}", context, source.to_string().red().bold()),
            RustiqueError::VersionError {context, source} => write!(f, "Version Parse Error: {}, {}", context, source.to_string().red().bold()),
            RustiqueError::NoVersionFound(e) => write!(f, "No Version Found: {}", e.to_string().red().bold()),
            RustiqueError::ModNotZipped(e) => write!(f, "Expected .zip, found folder. Did you forget to zip your mod? {}", e.to_string().yellow().bold()),
            RustiqueError::ConfigFileError(e) => write!(f, "Config File Error: {}", e.to_string().red().bold()),
            RustiqueError::MalformedModInfoJson(e) => write!(f, "Malformed {FILE_MODINFO_JSON} discovered for {}: Please contact the mod author. Rustique cannot process this mod.", e.to_string().red().bold()),
            RustiqueError::TomlError { context, source } => write!(f, "{}: {}", context, source.to_string().red().bold())
        }
    }
}

impl std::error::Error for RustiqueError {}

impl From<std::io::Error> for RustiqueError {
    fn from(e: std::io::Error) -> Self {
        RustiqueError::IoError {
            source: e,
            context: String::new()
        }
    }
}

impl From<url::ParseError> for RustiqueError {
    fn from(e: url::ParseError) -> Self {
        RustiqueError::UrlParseError(e)
    }
}