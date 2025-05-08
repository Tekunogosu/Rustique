use clap::{ArgGroup, Args, Subcommand};

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

    /// Show a specific option and its value
    Show(ShowArgs),

    /// Deletes an option, returning it to the default value
    Del(BoolArgs),
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

    #[arg(short, long)]
    pub notify_of_unzipped_mods: Option<bool>,

    /// The highest game version Rustique will use to download mods
    #[arg(short, long, value_name = "GAME_VERSION")]
    pub pin_game_version: Option<String>,

    /// Backup your mods before updating, preserves older versions
    #[arg(short, long)]
    pub backup_mods: Option<bool>,

    /// Directory for mod backups
    #[arg(short = 'B', long, value_name = "DIR")]
    pub backup_mods_dir: Option<String>,

    /// Rustique will attempt to identify mods that are not zipped and zip them for you.
    #[arg(short, long)]
    pub zip_mod_dirs: Option<bool>,

    /// Displays how long a command takes to complete
    #[arg(short, long)]
    pub show_execution_time: Option<bool>,
}

#[derive(Args, Debug)]
pub struct SetArgs {
    // #[command(flatten)]

    #[command(flatten)]
    pub common: CommonArgs
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    #[command(flatten)]
    pub common: BoolArgs
}

#[derive(Args, Debug)]
pub struct BoolArgs {

    #[arg(short, long)]
    pub mods_dir: bool,

    /// The highest game version Rustique will use to download mods
    #[arg(short, long)]
    pub pin_game_version: bool,

    /// Backup your mods before updating, preserves older versions
    #[arg(short, long)]
    pub backup_mods: bool,

    /// Directory for mod backups
    #[arg(short = 'B', long)]
    pub backup_mods_dir: bool,

    /// Rustique will attempt to identify mods that are not zipped and zip them for you.
    #[arg(short, long)]
    pub zip_mod_dirs: bool,
}

