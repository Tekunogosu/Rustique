use clap::{Args, Subcommand};
use crate::commands::arg_structs::info_args::ModInfoArgs;
use crate::commands::arg_structs::list_args::{ListArgs, ListOutputArgs};

#[derive(Args, Debug)]
pub struct ModpackCommands {
   #[command(subcommand)] 
   pub subcommand: ModpackSubCommands,
}


#[derive(Subcommand, Debug, Clone)]
pub enum ModpackSubCommands {
    
    /// Create a new mod pack. For a better guide, check the wiki: https://github.com/Tekunogosu/Rustique/wiki/Modpacks
    ///
    /// When you create a new modpack, the pack file itself will be save to ~/.config/rustique/modpacks/mypacks/yourmod.zip
    /// 
    /// The mods found in the mod_dir you are creating from, will be MOVED into ~/.config/rustique/modpacks/installed/yourmod.
    /// If you want to COPY the files instead, pass the --copy-mods flag with create. 
    /// 
    /// You can include your ModConfigs as well for a completely tailored modpack by adding --include-configs with create.
    /// By the configs will be MOVED unless you also add --copy-configs 
    /// 
    /// Once your mod has been created, you can use modpack local commands to manage your modpacks, see `Rustique modpack help local`
    /// 
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
    
    /// Syncs the modpacks sync file. 
    Sync,
    
    /// Show all modpacks that are currently installed
    List(ListArgs),
    
    /// Displays a nice table showing information about the modpack, including descriptions of each mod.
    Info(ModInfoArgs),
    
    /// Manipulate the modpacks you created
    Local(MPLocalArgs)
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Debug, Clone)]
pub struct MPCreateArgs {

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
    
    /// *Optional* By default, when you create a modpack it will grab ALL mods in the specified dir, including mods from other modpacks you have enabled. If you want to ignore the installed modpacks, set this to true.
    /// 
    /// This is false be default so you can make modpacks from other modpacks with ease. You can also just disable the modpacks first, but this option is available
    #[arg(short = 'I', long, default_value = "false")]
    pub ignore_modpacks: bool,
    
    /// *Optional* Copy the mods in your pack instead of moving them. By default, when you create a new modpack, the mods themselves will be moved into a new folder associated with your modpack. 
    #[arg(short = 'C', long, default_value = "false")]
    pub copy_mods: bool,

    // /// *Optional* Includes ALL configs found in VintagestoryData/ModConfig. Before you enable this, make sure ALL the configs are the ones you want.
    // #[arg(short = 'i', long, default_value = "false")]
    // pub include_configs: bool,
    // 
    // /// *Optional* Use with --include_configs. By default --include-configs MOVES the configs, this flag will COPY instead.
    // #[arg(short = 'G', long, requires = "include_configs", default_value = "false")]
    // pub copy_configs: bool,
}


#[derive(Args, Debug, Clone)]
pub struct MPInstallArgs {
    /// This is the ID from the mods website. This can either be the numerical ModID or the text version. Use `Rustique search -q modpackname` to get the numerical ID if you need
    pub mod_id: String,
    
    #[arg(short, long, default_value = "false")]
    pub missing_dependencies: bool,
   
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
pub struct MPInfo {}


#[derive(Args, Debug, Clone)]
pub struct MPLocalArgs {
    #[command(subcommand)]
    pub subcommands: MPLocalSubCommands,


}

#[derive(Subcommand, Debug, Clone)]
pub enum MPLocalSubCommands {
    /// List all locally created modpacks. It ONLY shows the modpacks you've created, use `Rustique modpack list` to see the modpacks you installed.
    List(MPLocalListOutputArgs),
    
    /// This feature is being worked on...
    Delete,
}

#[derive(Args, Debug, Clone)]
pub struct MPLocalListOutputArgs {
    #[clap(flatten)]
    pub output_commands: ListOutputArgs
}

