use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;

#[cfg(windows)]
use std::process::Command;

#[cfg(unix)]
use crate::information_utils::notice;

#[cfg(unix)]
use comfy_table::{Attribute, Color};

use async_zip::tokio::read::fs::ZipFileReader;
use futures::AsyncReadExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::info;
use uuid::Uuid;
use owo_colors::OwoColorize;
use tokio::fs;
use crate::api::client::ApiClient;
use crate::commands::download::download_file;
use crate::rustique_errors::RustiqueError;
use crate::traits::ref_ext::{PathRef, StrRef};



pub struct RustiqueUpdater {
    /// This is the name of the binary inside the archive
    pub new_binary_name: String,
    pub current_binary_path: PathBuf,
    /// This will be the temp working dir
    pub temp_dir: PathBuf,
    /// The full path to the download archive
    pub downloaded_path: PathBuf,
}


impl RustiqueUpdater {
    pub async fn new(binary_name: &str) -> Result<Self, RustiqueError> {

        let temp_dir = env::temp_dir().join(Uuid::new_v4().to_string());
        if !temp_dir.exists() {
            tokio::fs::create_dir_all(&temp_dir).await?;
        }
        
        info!("Created temp dir {}", temp_dir.display().magenta());

        Ok(Self {
            new_binary_name: binary_name.to_string(),
            temp_dir,
            downloaded_path: PathBuf::default(),
            current_binary_path: env::current_exe()?,
        })
    }


    pub async fn download_archive(&mut self, archive_name: &str, download_url: &str, finish_msg: impl StrRef) -> Result<&RustiqueUpdater, RustiqueError> {
        let client = ApiClient::new();
        download_file(&client, download_url, &self.temp_dir.join(archive_name), finish_msg).await?;
       
        self.downloaded_path = self.temp_dir.join(archive_name);
        
        match &self.extract_binary().await {
            // do cleanup if extraction fails
            Ok(_) => {}
            Err(e) => {
                info!("{}: {}","Failed to extract binary.. cleanup temp files".yellow(), e.red().bold());
                fs::remove_file(&self.temp_dir.join(archive_name)).await?;
                fs::remove_dir(&self.temp_dir).await?;
            }
        };

        Ok(self)
    }

    pub async fn extract_binary(&self) -> Result<PathBuf, RustiqueError> {
       
        info!("Extracting {}", &self.downloaded_path.display());
        
        let zip = ZipFileReader::new(&self.downloaded_path).await.map_err(|e| RustiqueError::ZipError {
            context: format!("Failed to load zip archive into ZipFileReader: {}", &self.downloaded_path.display()),
            source: e
        })?;
        
        info!("Looking for binary in archive called: {}", &self.new_binary_name);

        let entry_index = zip.file().entries().iter().position(|entry| {
            let filename = entry.filename().as_str().unwrap_or("");
            info!("Current file in archive: {}", filename.magenta());
            filename == self.new_binary_name || filename.ends_with(&format!("/{}", &self.new_binary_name))
        }).ok_or_else(|| RustiqueError::SimpleError(format!("Failed to locate {} in zip", &self.new_binary_name)))?;

        // extract the binary
        let mut entry_reader = zip.reader_with_entry(entry_index).await.map_err(|e| RustiqueError::ZipError {
            context: format!("Failed to create entry_reader for {}", &self.new_binary_name),
            source: e
        })?;

        let mut output_file = File::create(&self.temp_dir.join(&self.new_binary_name)).await?;
        let mut buffer = Vec::new();
        entry_reader.read_to_end(&mut buffer).await?;
        output_file.write_all(&buffer).await?;

        info!("{}","Successfully extracted binary from zip archive".green());

        Ok(self.temp_dir.join(&self.new_binary_name))
    }

    #[cfg(unix)]
    pub async fn update(&self) -> Result<(), RustiqueError> {
        // set the permissions of the new file from the old exe
        self.set_new_perms().await?;

        let exe_backup = &self.make_backup().await?;
       
        info!("Before copy/delete");
        // delete the current binary, we already have a backup
        fs::remove_file(&self.current_binary_path).await?;
        info!("After copy/delete");

        match fs::copy(&self.temp_dir.join(&self.new_binary_name), &self.current_binary_path).await {
            Ok(_) => {
                notice("Update successful!", Some(Color::Green), vec![Attribute::Bold]); 
                // update successful
                fs::remove_file(exe_backup).await?; // delete backup
                fs::remove_file(&self.downloaded_path).await?; // zip archive
                fs::remove_file(&self.temp_dir.join(&self.new_binary_name)).await?; // the extracted binary - since we copy it
                fs::remove_dir(&self.temp_dir).await?; // then the temp dir itself, even though this would be deleted on reboot
                Ok(())
            }
            Err(e) => {
                notice("Update Failed. Rolling back update.", Some(Color::Red), vec![Attribute::Bold]);  
                let _ = &self.restore_backup(exe_backup);
                Err(RustiqueError::SimpleError(format!("Update failed, restoring backup.. {e}")))
            }
        }
    }

    #[cfg(unix)]
    /// Sets the permissions of the temp_dir.join(binary_name)
    pub async fn set_new_perms(&self) -> Result<(), RustiqueError> {

        use std::os::unix::fs::PermissionsExt;
      
        info!("Attempting to get perms from the existing binary");
        let mut perms = fs::metadata(&self.temp_dir.join(&self.new_binary_name)).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&self.temp_dir.join(&self.new_binary_name), perms).await?;
        

        info!("Permissions copied from current binary to new one");
        Ok(())
    }
    
    #[cfg(windows)]
    pub async fn create_update_script(&self) -> Result<&RustiqueUpdater, RustiqueError> {
        
        let script_path = self.temp_dir.join("update.bat");
        
        let exe_backup = &self.make_backup().await?;
        
        // load update script and replace placeholders
        let template = include_str!("windows_updater.bat");
        let script_content = template
            .replace("{EXE_NAME}", self.get_current_binary_filename()?.to_str().unwrap())
            .replace("{CURRENT_EXE}", self.current_binary_path.to_string_lossy().as_ref())
            .replace("{BACKUP_PATH}", exe_backup.to_string_lossy().as_ref())
            .replace("{NEW_BINARY}", self.temp_dir.join(&self.new_binary_name).to_string_lossy().as_ref());
        
        fs::write(&script_path, &script_content).await?;
        
        Ok(self)
    }
    
    #[cfg(windows)]
    pub fn execute_update_bat(&self) -> Result<(), RustiqueError> {
        info!("Exiting Rustique and executing update bat");
        
        // start the update script in background
        Command::new("cmd")
            .args(["/C", "start", "/MIN", self.temp_dir.join("update.bat").to_string_lossy().as_ref()])
            .spawn()
            .map_err(|e| RustiqueError::SimpleError(format!("{}: {}", "Failed to spawn windows update process".yellow(), e.to_string().red().bold())))?;
        
        // exit so the update script can be updated
        std::process::exit(0);
    }

    pub async fn make_backup(&self) -> Result<PathBuf, RustiqueError> {
        let backup_path = &self.temp_dir
            .join(self.get_current_binary_filename()?)
            .with_added_extension("backup");

        fs::copy(&self.current_binary_path, &backup_path).await?;

        Ok(backup_path.clone())
    }

    
    #[allow(dead_code)]
    pub async fn restore_backup(&self, backup_path: impl PathRef) -> Result<(), RustiqueError> {
        // with_extension("") to remove the .backup added
        fs::copy(backup_path, &self.current_binary_path.with_extension("")).await?;
        Ok(())
    }

    fn get_current_binary_filename(&self) -> Result<&OsStr, RustiqueError> {
        self.current_binary_path
            .file_name()
            .ok_or_else(|| RustiqueError::SimpleError("Unable to get file name from current exe path".into()))
    }

}