use clap::{Args, Subcommand};
use crate::commands::arg_structs::info_args::ModInfoArgs;

#[derive(Args, Debug)]
pub struct ModpackCommands {
   #[command(subcommand)] 
   pub subcommand: ModpackSubCommands,
}


#[derive(Subcommand, Debug, Clone)]
pub enum ModpackSubCommands {
    
    /// Create a new mod pack.  
    #[command(about = "Create a new modpack")]
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
    
    /// Displays a nice table showing informatio about the modpack, including descriptions of each mod.
    Info(ModInfoArgs)
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
    
    /// *Optional* Location to save modpack, default is ~/.config/rustique/modpacks/mypacks
    #[arg(short, long, value_name = "PATH")]
    pub save_path: Option<String>,
}


#[derive(Args, Debug, Clone)]
pub struct MPInstallArgs {
    /// This is the ID from the mods website. This can either be the numerical ModID or the text version. Use `Rustique search -q modpackname` to get the numerical ID if you need
    pub mod_id: String,
   
    /// Download a specific version of the modpack
    #[arg(short = 'v', long, value_name = "VERSION")]
    pub mod_version: Option<String>
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
    /// ID of the modpack to enable. See `Rustique modpack list` to get the correct modpack ID if you don't know it.
    pub mpk_id: String,
    
    /// --force allows you to enable multiple modpacks at once. NOTE: You may make your world unstable by having modpacks with conflicting mods. There is no way for Rustique to check this, you have been warned.
    #[arg(short, long, default_value = "false")]
    pub force: bool,
}

#[derive(Args, Debug, Clone)]
pub struct MPListArgs {
    #[arg(short, long, default_value = "false")]
    pub show_only_updates: bool,
}

#[derive(Args, Debug, Clone)]
pub struct MPInfo {}
