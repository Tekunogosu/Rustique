use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::aliases::ModID;
use crate::api::api_structs::ModInfo;
use crate::rustique_errors::RustiqueError;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModPackToml {
    
    pub modpack: ModPack,
    pub mods: HashMap<ModID,MPMods>,
}

impl ModPackToml {
    pub fn save(&self, save_path: &PathBuf, ) -> Result<(), RustiqueError> {
        
        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| RustiqueError::SimpleError(format!("Failed in modpack toml save {e}")))?;
        
        
        File::create(save_path)
            .map_err(|e| RustiqueError::SimpleError(format!("Failed to create modpack toml {e}")))?
            .write_all(toml_content.as_bytes())
            .map_err(|e| RustiqueError::SimpleError(format!("Failed to write modpack toml {e}")))?;
        
        Ok(())
    }
    
    pub fn gen_modinfo_json(&self, save_path: &PathBuf) -> Result<(), RustiqueError> {
       
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
        
        mod_info.dependencies = Some(self.mods.values().map(|mp_mod| (mp_mod.mod_id.clone(), mp_mod.version.clone())).collect());
        
        
        println!("{mod_info:#?}");
        
        let file_name = "modinfo.json";
        
        let mut file = File::create(save_path.join(file_name))?;
        
        file.write_all(serde_json::to_string_pretty(&mod_info)
            .map_err(|e| RustiqueError::SimpleError(format!("Failed writing the mod_info of the modpack, {e}")))?
            .as_bytes())?;
        
        
        Ok(())
    }
    
    pub fn create_modpack_zip(&self, save_path: &PathBuf) -> Result<(), RustiqueError> {
        
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

