
#[cfg(unix)]
use tokio::fs::symlink;

#[cfg(windows)]
use tokio::fs::{symlink_file, symlink_dir};

use std::fs;
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::PathRef;

pub struct SymlinkManager;

impl SymlinkManager {
   
    /// Manage symlink creation
    pub async fn create(target: impl PathRef, link: impl PathRef) -> Result<(), RustiqueError> {
        let (target, link) = (target.as_ref(), link.as_ref());
        #[cfg(unix)]
        symlink(target, link).await.map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        
        #[cfg(windows)]
        if target.is_dir() {
            symlink_dir(target, link).await.map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        } else {
            symlink_file(target, link).await.map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    pub fn remove(path: impl PathRef) -> Result<(), RustiqueError> {
        fs::remove_file(path.as_ref())
            .map_err(|e| RustiqueError::SimpleError(e.to_string()))?;
        
        Ok(())
    }
    
    
    /// Checks if `path` is a symlink
    pub fn exists(path: impl PathRef) -> bool {
        path.as_ref().is_symlink()
    }
}