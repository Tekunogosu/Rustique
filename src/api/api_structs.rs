use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::Write;
use zip::{CompressionMethod, ZipWriter};
use zip::write::SimpleFileOptions;
use crate::aliases::{FileName, ModID, ModVersion};
use crate::consts::FILE_MODINFO_JSON;
use crate::information_utils::{command_output, display_table};
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::PathRef;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq)]
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
        write!(f, "{str}")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq)]
#[serde(untagged)]
pub enum StringOrBool {
    String(String),
    Bool(bool),
}

impl Default for StringOrBool {
    fn default() -> Self {
        StringOrBool::String(String::new())
    }
}

impl Display for StringOrBool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            StringOrBool::String(s) => s.clone(),
            StringOrBool::Bool(b) => b.to_string(),
        };
        write!(f, "{str}")
    }
}

// Due to mod authors not following the modinfo.json spec for mods, we have to
// put an alias for all fields found in modinfo.json file.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct ModInfo {
    #[serde(default, alias = "Name")]
    pub name: String,

    // StringOrInt is required because there are a few mod authors that use and Int
    // instead of string for the type
    #[serde(default, rename = "type", alias = "Type")]
    pub mod_type: StringOrInt,

    #[serde(default, rename = "modid", alias = "modId", alias = "ModId", alias = "ModID", alias = "modID", alias = "mod_id", alias = "Mod_id", alias = "Mod_ID", alias = "Mod_Id", alias = "MOD_ID")]
    pub mod_id: ModID,
    #[serde(default, alias = "Version")]
    pub version: Option<ModVersion>,
    #[serde(default, rename = "networkVersion", alias = "NetworkVersion", alias = "Networkversion", alias = "networkversion")]
    pub network_version: Option<String>,
    #[serde(default, rename = "textureSize", alias = "TextureSize", alias = "Texturesize", alias = "texturesize")]
    pub texture_size: Option<u32>,
    #[serde(default, alias = "Description")]
    pub description: Option<String>,
    #[serde(default, alias = "Website")]
    pub website: Option<String>,
    #[serde(default, alias = "Authors")]
    pub authors: Vec<String>,
    #[serde(default, alias = "Contributors")]
    pub contributors: Vec<String>,
    #[serde(default, alias = "Side")]
    pub side: Option<String>,
    #[serde(default, rename = "requiredOnClient", alias = "RequiredOnClient", alias = "RequiredonClient", alias = "Requiredonclient", alias = "requiredonclient")]
    pub required_on_client: Option<StringOrBool>,
    #[serde(default, rename = "requiredOnServer", alias = "RequiredOnServer", alias = "RequiredonServer", alias = "Requiredonserver", alias = "requiredonserver")]
    pub required_on_server: Option<StringOrBool>,
    #[serde(default, alias = "Dependencies")]
    pub dependencies: HashMap<ModID, ModVersion>,
}

impl ModInfo {
   pub fn build_modpack(&self, save_path: impl PathRef, mpk_id: FileName) -> Result<(), RustiqueError> {
        // config dir should all be setup by this point
       
        let zip_path = save_path.as_ref().join(mpk_id +".zip");
        let zip_archive = File::create(&zip_path)?;
        let mut zip = ZipWriter::new(zip_archive);
       
        // Compression needs to be set to Deflated to make it the most compatible
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);
       
       
        let mod_info = serde_json::to_string_pretty(&self)
            .map_err(|e|RustiqueError::SimpleError(e.to_string()))?;
        self.add_file_to_zip(&mut zip, FILE_MODINFO_JSON, &mod_info, options).inspect_err(|_| {
            let _ = self.delete_zip(&zip_path);
        })?;
        
        
        zip.finish().map_err(|e| {
            let _ = self.delete_zip(&zip_path);
            RustiqueError::ZipError {
                context: "Failed creating modpack zip".into(),
                source: e
            }
        })?;
        
        
        display_table(
            vec![command_output("Your Modpack has been created and saved to", zip_path.to_string_lossy())], 
            None);

        Ok(())
    } 
    
     fn delete_zip(&self, save_path: impl PathRef) -> Result<(), RustiqueError> {
        fs::remove_file(save_path.as_ref())
            .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        
        Ok(())
    }
    
    fn add_file_to_zip(&self, zip: &mut ZipWriter<File>, filename: &str, content: &str, options: SimpleFileOptions) -> Result<(), RustiqueError> {
        zip.start_file(filename, options)
            .map_err(|e| RustiqueError::ZipError { context: format!("create: {filename}"), source: e })?;
        zip.write_all(content.as_bytes())
            .map_err(|e| RustiqueError::SimpleError(format!("Failed to write to zip archive {}",e.to_string())))?;
        Ok(())
    } 
    
}

// Used for endpoint /api/mods
#[derive(Serialize, Deserialize, Debug)]
pub struct Mods {
    pub mods: Vec<ModApi>,
    #[serde(default, rename = "statuscode")]
    pub status_code: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModsSearchFile {
    pub mods: Vec<ModApi>,
    pub last_sync: String
}

impl ModsSearchFile {
    pub fn new() -> Self {
        Self {
            mods: Vec::new(),
            last_sync: String::new()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[derive(Default)]
pub struct ModApi {
    #[serde(default, rename = "modid", alias = "mod_id")]
    pub mod_id: u32,
    #[serde(default, rename = "assetid")]
    pub asset_id: u32,
    pub downloads: u32,
    pub follows: u32,
    #[serde(default, rename = "trendingpoints")]
    pub trending_points: u32,
    pub comments: u32,
    pub name: Option<String>,
    pub summary: Option<String>,
    #[serde(default, rename = "modidstrs")]
    pub mod_id_strs: Vec<String>,
    pub author: Option<String>,
    #[serde(default, rename = "urlalias")]
    pub url_alias: Option<String>,
    pub side: Option<String>,
    #[serde(default, rename = "type")]
    pub mod_type: Option<String>,
    pub logo: Option<String>,
    pub tags: Vec<String>,
    #[serde(default, rename = "lastreleased")]
    pub last_released: Option<String>
}


// Used for endpoint /api/mod/mod_id
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Mod {
    #[serde(rename="mod", alias = "Mod")]
    pub mod_json: ApiModJson,

    #[serde(default, rename="statuscode")]
    pub status_code: String
}


// Used with api/mod/modid
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct ApiModJson {
    #[serde(default, rename="modid", alias = "ModID", alias = "mod_id")]
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
    pub logo_file: Option<String>,

    #[serde(default, rename = "logofiledb",skip_serializing_if = "Option::is_none")]
    pub logo_file_db: Option<String>,

    #[serde(default, rename = "homepageurl", skip_serializing_if = "Option::is_none")]
    pub home_page_url: Option<String>,

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
    
    #[serde(default)]
    pub trending_points: u32,
    
    #[serde(default)]
    pub comments: u32,
    
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
    pub tags: Vec<String>,
    #[serde(default)]
    pub releases: Vec<Releases>,
    #[serde(default)]
    pub screenshots: Vec<Screenshots>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Releases {
    #[serde(default, rename = "release_id")]
    pub release_id: u32,
    #[serde(default, rename = "mainfile")]
    pub main_file: Option<String>,

    // mod awearablelight has an int for a filename..
    #[serde(default)]
    pub filename: Option<StringOrInt>,
    // mod awearablelight has null for a fileid on a release
    #[serde(default, rename = "fileid")]
    pub file_id: Option<u32>,

    #[serde(default)]
    pub downloads: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, rename = "modidstr")]
    pub modid_str: Option<String>,
    #[serde(default, rename = "modversion")]
    pub mod_version: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
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
    pub game_versions: Vec<GameVersion>,
}

#[derive(Deserialize, Serialize, Debug)]
#[derive(PartialEq)]
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