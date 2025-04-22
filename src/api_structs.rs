use std::collections::HashMap;
use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum StringOrInt {
    String(String),
    Int(i64),
}

impl Default for StringOrInt {
    fn default() -> Self {
        StringOrInt::String(String::new())
    }
}

impl Display for StringOrInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            StringOrInt::String(s) => s.clone(),
            StringOrInt::Int(i) => i.to_string(),
        };
        write!(f, "{}", str)
    }
}

// Due to mod authors not following the modinfo.json spec for mods, we have to
// put an alias for all fields found in modinfo.json file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModInfo {
    #[serde(default, alias = "Name")]
    pub name: String,

    // StringOrInt is required because there are a few mod authors that use and Int
    // instead of string for the type
    #[serde(default, rename = "type", alias = "Type")]
    pub mod_type: StringOrInt,

    #[serde(default, rename = "modid", alias = "modId", alias = "ModId", alias = "ModID", alias = "modID")]
    pub mod_id: String,
    #[serde(default, alias = "Author")]
    pub version: Option<String>,
    #[serde(default, rename = "networkVersion", alias = "NetworkVersion", alias = "Networkversion", alias = "networkversion")]
    pub network_version: Option<String>,
    #[serde(default, rename = "textureSize", alias = "TextureSize", alias = "Texturesize", alias = "texturesize")]
    pub texture_size: Option<u32>,
    #[serde(default, alias = "Description")]
    pub description: Option<String>,
    #[serde(default, alias = "Website")]
    pub website: Option<String>,
    #[serde(default, alias = "Authors")]
    pub authors: Option<Vec<String>>,
    #[serde(default, alias = "Contributors")]
    pub contributors: Option<Vec<String>>,
    #[serde(default, alias = "Side")]
    pub side: Option<String>,
    #[serde(default, rename = "requiredOnClient", alias = "RequiredOnClient", alias = "RequiredonClient", alias = "Requiredonclient", alias = "requiredonclient")]
    pub required_on_client: Option<bool>,
    #[serde(default, rename = "requiredOnServer", alias = "RequiredOnServer", alias = "RequiredonServer", alias = "Requiredonserver", alias = "requiredonserver")]
    pub required_on_server: Option<bool>,
    #[serde(default, alias = "Dependencies")]
    pub dependencies: Option<HashMap<String, String>>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Mod {
    #[serde(rename="mod")]
    pub mod_json: ApiModJson,

    #[serde(default, rename="statuscode")]
    pub status_code: Option<String>
}


#[derive(Deserialize, Serialize, Debug)]
pub struct ApiModJson {
    #[serde(default, rename="modid")]
    pub mod_id: u32,
    #[serde(default, rename="assetid")]
    pub asset_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, rename = "urlalias", skip_serializing_if = "Option::is_none")]
    pub url_alias: Option<String>,

    #[serde(default, rename = "logofilename", skip_serializing_if = "Option::is_none")]
    pub logo_filename: Option<String>,

    #[serde(default, rename = "logofile", skip_serializing_if = "Option::is_none")]
    pub logo_file: Option<Option<String>>,

    #[serde(default, rename = "logofiledb",skip_serializing_if = "Option::is_none")]
    pub logo_file_db: Option<Option<String>>,

    #[serde(default, rename = "homepageurl", skip_serializing_if = "Option::is_none")]
    pub home_page_url: Option<Option<String>>,

    #[serde(default, rename = "sourcecodeurl", skip_serializing_if = "Option::is_none")]
    pub source_code_url: Option<String>,

    #[serde(default, rename = "trailervideourl", skip_serializing_if = "Option::is_none")]
    pub trailer_video_url: Option<String>,

    #[serde(default, rename = "issuetrackerurl", skip_serializing_if = "Option::is_none")]
    pub issue_tracker_url: Option<String>,

    #[serde(default, rename = "wikiurl", skip_serializing_if = "Option::is_none")]
    pub wiki_url: Option<String>,

    #[serde(default)]
    pub downloads: u32,
    #[serde(default)]
    pub follows: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,

    #[serde(default, rename = "type")]
    pub mod_type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(default, rename = "lastreleased",skip_serializing_if = "Option::is_none")]
    pub last_released: Option<String>,
    #[serde(default, rename = "lastmodified",skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub tags: Vec<Option<String>>,
    #[serde(default)]
    pub releases: Vec<Releases>,
    #[serde(default)]
    pub screenshots: Vec<Screenshots>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Releases {
    #[serde(default, rename = "release_id")]
    pub release_id: u32,
    #[serde(default, rename = "mainfile")]
    pub main_file: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default, rename = "fileid")]
    pub file_id: u32,
    #[serde(default)]
    pub downloads: u32,
    #[serde(default)]
    pub tags: Vec<Option<String>>,
    #[serde(default, rename = "modidstr")]
    pub modid_str: Option<String>,
    #[serde(default, rename = "modversion")]
    pub mod_version: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Screenshots {
    #[serde(default, rename = "fileid")]
    pub file_id: u32,
    #[serde(default, rename = "mainfile")]
    pub main_file: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default, rename = "thumbnailfilename")]
    pub thumbnail_filename: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
}


// /api/tags
#[derive(Deserialize, Serialize, Debug)]
pub struct Tags {
    #[serde(default, rename = "statuscode")]
    pub status_code: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Tag {
    #[serde(default, rename = "tagid")]
    pub tag_id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: String,
}


// /api/gameversions
#[derive(Deserialize, Serialize, Debug)]
pub struct GameVersions {
    #[serde(default, rename = "statuscode")]
    pub status_code: String,
    #[serde(default, rename = "gameversions")]
    pub game_versions: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GameVersion {
    #[serde(default, rename = "tagid")]
    pub tag_id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: String,
}


// /api/authors

#[derive(Deserialize, Serialize, Debug)]
pub struct Authors {
    #[serde(default, rename = "statuscode")]
    pub status_code: String,
    #[serde(default)]
    pub authors: Vec<Author>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Author {
    #[serde(default, rename = "userid")]
    pub user_id: u32,
    #[serde(default)]
    pub name: String,
}

// /api/comments/{modid (optional)}
#[derive(Deserialize, Serialize, Debug)]
pub struct Comments {
    #[serde(default, rename = "statuscode")]
    pub status_code: String,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Comment {
    #[serde(default, rename = "commentid")]
    pub comment_id: u32,
    #[serde(default, rename = "assetid")]
    pub asset_id: u32,
    #[serde(default, rename = "userid")]
    pub user_id: u32,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub created: String,
    #[serde(default, rename = "lastmodified")]
    pub last_modified: String,
}


// /api/changelogs/{modid (optional)}
#[derive(Deserialize, Serialize, Debug)]
pub struct ChangeLogs {
    #[serde(default, rename = "statuscode")]
    pub status_code: String,
    #[serde(default, rename = "changelogs")]
    pub changelogs: Vec<ChangeLog>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ChangeLog {
    #[serde(default, rename = "changelogid")]
    pub changelog_id: u32,
    #[serde(default, rename = "assetid")]
    pub asset_id: u32,
    #[serde(default, rename = "userid")]
    pub user_id: u32,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub created: String,
    #[serde(default, rename = "lastmodified")]
    pub last_modified: String,
}