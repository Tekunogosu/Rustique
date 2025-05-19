use clap::{Args, Subcommand};

#[derive(Subcommand, Debug, Clone)]
pub enum ModpackCommands {
    
    /// Create a new mod pack.  
    Create(MPCreateArgs),

    /// Installing a modpack is like installing a mod, except it is treated differently with Rustique.
    ///
    /// Rustique will create a new alias for the modpack and a directory to store the mods associated.
    /// This makes it easier to manage multiple modpacks without affecting the entire mod system
    ///
    /// By default all modpacks are installed to ~/.config/rustique/modpacks/alias_name
    Install(MPInstallArgs),

    /// Update a specific modpack, you *can* use the normal way to update, but this command checks the modpacks page for updates instead of individual mods themselves.
    Update(MPUpdateArgs),

    /// Enabling a modpack starts by moving the existing Mods directory *if not a modpack itself* to ~/.config/rustique/modpacks/backup/date-time/{Mods,ModData,ModConfig}
    ///
    /// Followed by moving the modpack directory into the old mods place.
    ///
    Enable(MPEnableArgs),
    
    /// Disabling a modpack does the opposite of enable. First it moves the modpack back to its modpack directory,
    /// then moves the most recently moved `Mods-date` folder back into place ~/.config/VintagestoryData/Mods5
    Disable(MPDisableArgs),
    
    /// Delete a modpack by its id (alias)
    Delete(MPDeleteArgs),
    
    /// Show all modpacks that are currently installed
    List(MPListArgs),
}

#[derive(Args, Debug, Clone)]
pub struct MPCreateArgs {
    
    /// There are a lot of options, this flag will ask you all the questions to create a new mod pack
    #[arg(short, long, default_value = "false")]
    pub interactive: bool,
    
    /// *Required* This is the long form name of your modpack, "Theysa magic mod pack"
    #[arg(short, long)]
    pub name: String,
    
    /// *Required* This is the ID rustique will use, "theysa_magicmpk". This is also the id that should be set as the url on the mods website, so they match
    #[arg(short, long)]
    pub mpk_id: String,
    
    /// *Required* This is the version of your modpack. Needed to keep track of updates.
    #[arg(short = 'v', long)]
    pub mpk_version: String,
    
    /// *Optional* This pins the game version so only mods that declared themselves compatible will be pulled into the pack. Default is set to the latest stable game version
    /// 
    /// Use this with caution as many mods do not specify accurate game versions. 
    #[arg(short, long)]
    pub game_version: Option<String>,
    
    /// *Optional* Description of your mod pack
    #[arg(short, long)]
    pub description: Option<String>,
    
    /// *Optional* Author
    #[arg(short, long)]
    pub author: Option<String>,
    
    /// *Optional* Contact
    #[arg(short, long)]
    pub contact: Option<String>,
    
    /// *Optional* Website
    #[arg(short, long)]
    pub website: Option<String>,
}


#[derive(Args, Debug, Clone)]
pub struct MPInstallArgs {
    /// This is the ID from the mods website. This can either be the numerical ModID or the text version. Use `Rustique search -q modpackname` to get the numerical if you need
    pub mod_id: String,
    
    /// This overrides the alias the mod author provides with one of your own. You'll be able to use this alias to modify the modpack once its installed.
    pub alias: Option<String>,
}


#[derive(Args, Debug, Clone)]
pub struct MPUpdateArgs {
    pub mpk_id: String,
}

#[derive(Args, Debug, Clone)]
pub struct MPDeleteArgs {
    pub mpk_id: String,
}

#[derive(Args, Debug, Clone)]
pub struct MPDisableArgs {
    pub mpk_id: String,
}

#[derive(Args, Debug, Clone)]
pub struct MPEnableArgs{
    pub mpk_id: String,
}

#[derive(Args, Debug, Clone)]
pub struct MPListArgs {}
