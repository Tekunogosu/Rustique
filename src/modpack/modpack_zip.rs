use std::collections::HashMap;
use std::fs;
use zip::{ZipWriter, CompressionMethod};
use std::fs::File;
use std::io::Write;
use serde::{Deserialize, Serialize};
use tracing::debug;
use zip::write::SimpleFileOptions;
use crate::aliases::{FileName, ModID, ModName};
use crate::api::api_structs::ModInfo;
use crate::consts::{FILE_MODINFO_JSON, FILE_MODPACK_TOML};
use crate::information_utils::{command_output, display_table};
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::PathRef;

// This file is likely to not be used, but for now we will keep it in




#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModPackZip {
   
    #[serde(default)]
    pub modpack: ModPack,
    #[serde(default)]
    pub mods: HashMap<ModName,MPMods>,
}

impl ModPackZip {
    /// Creates a ModInfo from the ModPackToml data
    pub fn gen_modinfo(&self) -> Result<ModInfo, RustiqueError> {
       
        let mut mod_info = ModInfo::default();
        
        mod_info.mod_id.clone_from(&self.modpack.mpk_id);
        mod_info.name.clone_from(&self.modpack.name);
        mod_info.version.clone_from(&Some(self.modpack.version.clone()));
        
        
        if let Some(author) = self.modpack.author.clone() {
            mod_info.authors = vec![author];
        }
        
        if let Some(desc) = self.modpack.description.clone() {
            mod_info.description = Some(desc);
        }
        
        if let Some(website) = self.modpack.website.clone() {
            mod_info.website = Some(website);
        }
        
        mod_info.dependencies = self.mods.values().map(|mp_mod| (mp_mod.mod_id.clone(), mp_mod.version.clone())).collect();
        
        debug!("{mod_info:#?}");
        
        Ok(mod_info)
    }
    
    pub fn build_modpack(&self, save_path: impl PathRef, modpack_id: FileName) -> Result<(), RustiqueError> {
        // config dir should all be setup by this point
       
        let zip_path = save_path.as_ref().join("mypacks").join(modpack_id +".zip");
        let zip_archive = File::create(&zip_path)?;
        let mut zip = ZipWriter::new(zip_archive);
       
        // Compression needs to be set to Deflated to make it the most compatible
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        
        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| RustiqueError::SimpleError(format!("Failed to make pretty modpack toml: {e}")))?;
        self.add_file_to_zip(&mut zip, FILE_MODPACK_TOML, &toml_content, options).inspect_err(|_| {
            let _ = self.delete_zip(&zip_path);
        })?;
        
        
        let mod_info = serde_json::to_string_pretty(&self.gen_modinfo()?)
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ModPack {
    pub name: String,
    pub mpk_id: String,
    pub version: String,
    
    #[serde(default)]
    pub game_version: Option<String>,
    
    #[serde(default)]
    pub description: Option<String>,
    
    #[serde(default)]
    pub author: Option<String>,
    
    #[serde(default)]
    pub contact: Option<String>,
    
    #[serde(default)]
    pub website: Option<String>,
    
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MPMods {
    pub mod_id: ModID,
    pub version: String,
}

