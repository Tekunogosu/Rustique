use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use tokio::fs::File;
use async_zip::tokio::write::ZipFileWriter;
use async_zip::ZipEntryBuilder;
use crate::aliases::{FileName, ModID, ModVersion};
use crate::consts::FILE_MODINFO_JSON;
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

fn some_array_items<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(d).map(|x: Vec<Option<String>>| {
        x.iter()
            .filter_map(|y: &Option<String>| y.is_some().then(|| y.clone().unwrap_or_default()))
            .collect()
    })
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
    pub texture_size: Option<i64>,
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
    pub async fn build_modpack(&self, save_path: impl PathRef, mpk_id: FileName) -> Result<PathBuf, RustiqueError> {
        // config dir should all be setup by this point
        
        let zip_path = save_path.as_ref().join(mpk_id +".zip");
        let zip_archive = File::create(&zip_path).await?;
        let mut zip = ZipFileWriter::with_tokio(zip_archive);
       
       
        // Compression needs to be set to Deflated to make it the most compatible
       
        let mod_info = serde_json::to_string_pretty(&self)
           .map_err(|e|RustiqueError::SimpleError(e.to_string()))?;
        
        if let Err(e) = self.add_file_to_zip(&mut zip, FILE_MODINFO_JSON, &mod_info, async_zip::Compression::Deflate).await {
            let _ = self.delete_zip(&zip_path).await;
           return Err(RustiqueError::SimpleError(format!("Unable to add file to zip archive {e}")));
        }
        
        if let Err(e) = zip.close().await { 
            let _ = self.delete_zip(&zip_path).await;
            return Err(RustiqueError::ZipError {
                context: "Failed creating modpack zip".into(),
                source: e
            });
        }
        
        
       Ok(zip_path) 
    } 
    
    async fn delete_zip(&self, save_path: impl PathRef) -> Result<(), RustiqueError> { 
        tokio::fs::remove_file(save_path.as_ref()).await
            .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn add_file_to_zip(&self, zip: &mut ZipFileWriter<File>, filename: &str, content: &str, compression: async_zip::Compression) -> Result<(), RustiqueError> {
        use async_zip::ZipEntryBuilder;
        
        let entry_builder = ZipEntryBuilder::new(filename.into(), compression);
        
        zip.write_entry_whole(entry_builder, content.as_bytes()).await
            .map_err(|e| RustiqueError::ZipError { 
                context: format!("Unable to create: {filename}"),
                source: e 
            })?;
        
        Ok(())
    }
   
    #[allow(dead_code)]
   async fn add_dir_to_zip(&self, zip: &mut ZipFileWriter<File>, dir_path: impl PathRef, zip_prefix: &str) -> Result<(), RustiqueError> {
        
        let mut entries = tokio::fs::read_dir(dir_path.as_ref()).await
            .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| RustiqueError::SimpleError(e.to_string()))? {
            
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let zip_path = if zip_prefix.is_empty() { 
                name.clone() 
            } else { 
                format!("{zip_prefix}/{name}") 
            };
            
            if path.is_dir() {
               Box::pin(self.add_dir_to_zip(zip, &path, &zip_path)).await?;
            } else {
                let content = tokio::fs::read(&path).await
                    .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
                
                let entry_builder = ZipEntryBuilder::new(zip_path.clone().into(), async_zip::Compression::Deflate);
                zip.write_entry_whole(entry_builder, &content).await
                    .map_err(|e| RustiqueError::ZipError {
                        context: format!("Unable to create: {zip_path}"),
                        source: e
                    })?;
            }
        }
       
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ModApi {
    #[serde(default, rename = "modid", alias = "mod_id")]
    pub mod_id: i64,
    #[serde(default, rename = "assetid")]
    pub asset_id: i64,
    pub downloads: i64,
    pub follows: i64,
    #[serde(default, rename = "trendingpoints")]
    pub trending_points: i64,
    pub comments: i64,
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
    pub mod_id: i64,
    #[serde(default, rename="assetid")]
    pub asset_id: i64,
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
    pub downloads: i64,
    
    #[serde(default)]
    pub follows: i64,
    
    #[serde(default, rename = "trendingpoints")]
    pub trending_points: i64,
    
    #[serde(default)]
    pub comments: i64,
    
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
    #[serde(default, deserialize_with="some_array_items")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub releases: Vec<Release>,
    #[serde(default)]
    pub screenshots: Vec<Screenshots>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Release {
    #[serde(default, rename = "release_id")]
    pub release_id: i64,
    #[serde(default, rename = "mainfile")]
    pub main_file: Option<String>,

    // mod awearablelight has an int for a filename..
    #[serde(default)]
    pub filename: Option<StringOrInt>,
    // mod awearablelight has null for a fileid on a release
    #[serde(default, rename = "fileid")]
    pub file_id: Option<i64>,

    #[serde(default)]
    pub downloads: i64,
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
    pub file_id: i64,
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
    pub tag_id: i64,
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
    pub tag_id: i64,
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
    pub user_id: i64,
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
    pub comment_id: i64,
    #[serde(default, rename = "assetid")]
    pub asset_id: i64,
    #[serde(default, rename = "userid")]
    pub user_id: i64,
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
    pub changelog_id: i64,
    #[serde(default, rename = "assetid")]
    pub asset_id: i64,
    #[serde(default, rename = "userid")]
    pub user_id: i64,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub created: String,
    #[serde(default, rename = "lastmodified")]
    pub last_modified: String,
}