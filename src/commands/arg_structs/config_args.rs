use clap::{ArgGroup, Args, Subcommand};
use crate::commands::arg_structs::config_table_args::TableArgs;

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub(crate) subcommand: ConfigSubCommand,
   
}

#[derive(Subcommand)]
pub enum ConfigSubCommand {

    /// Set a value in the config file
    Set(SetArgs),

    /// List all config options and their current values
    List,

    /// Deletes an option, returning it to the default value.
    /// You can set multiple values at the same time: Rustique config del -mzB
    Del(BoolArgs),
    
    /// Configure the tables for `List` and `Search` 
    Table(TableArgs),
}


#[derive(Args, Debug, Clone)]
#[command(group(
    ArgGroup::new("common_args_flags")
    .required(false)
))]
pub struct CommonArgs {

    /// Default mod directory Rustique will manage
    ///
    /// This path MUST be an absolute path
    ///
    /// Example: /home/username/.config/VintagestoryData/Mods
    ///
    /// You can use ~/ as well, it will expand into /home/username/
    ///
    /// Default: '~/.config/VintagestoryData/Mods' for Unix systems (Linux and Mac)
    ///          '%appdata%/Vintagestory/Mods' for windows
    #[arg(short, long)]
    pub mods_dir: Option<String>,

    /// Setting this to 'true' will show a message if you have unzipped mods in your mod dir
    #[arg(short, long)]
    pub notify_of_unzipped_mods: Option<bool>,

    /// The highest game version Rustique will use to download mods (WIP)
    ///
    /// Default: None
    #[arg(short, long, value_name = "GAME_VERSION")]
    pub pin_game_version: Option<String>,

    /// Backup your mods before updating, preserves older versions (WIP)
    ///
    /// Before Rustique updates any file, the old version will be copied to the --backup-mods-dir.
    ///
    /// Default: false
    ///
    #[arg(short, long)]
    pub backup_mods: Option<bool>,

    /// Directory for mod backups
    ///
    /// Default: (Linux): ~/.config/rustique/mod_backups
    ///
    ///          (Windows) %appdata%/rustique/mod_backups
    ///
    #[arg(short = 'B', long, value_name = "DIR")]
    pub backup_mods_dir: Option<String>,

    /// Rustique will attempt to identify mods that are not zipped and zip them for you. (WIP)
    #[arg(short, long)]
    pub zip_mod_dirs: Option<bool>,

    /// Displays how long a command takes to complete
    ///
    /// Default: true
    #[arg(short, long)]
    pub show_execution_time: Option<bool>,
}
#[derive(Args, Debug)]
pub struct SetArgs {
    #[command(flatten)]
    pub common: CommonArgs
}

#[derive(Args, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct BoolArgs {

    #[arg(short, long)]
    pub mod_dir: bool,

    /// The highest game version Rustique will use to download mods
    #[arg(short, long)]
    pub pin_game_version: bool,

    /// Backup your mods before updating, preserves older versions
    #[arg(short, long)]
    pub backup_mods: bool,

    /// Directory for mod backups
    #[arg(short = 'B', long)]
    pub backup_mods_dir: bool,

    #[arg(short, long)]
    pub notify_of_unzipped_mods: bool,

    /// Rustique will attempt to identify mods that are not zipped and zip them for you.
    #[arg(short, long)]
    pub zip_mod_dirs: bool,
}

