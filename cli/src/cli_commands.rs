use crate::commands::arg_structs::config_args::ConfigCommand;
use crate::commands::arg_structs::info_args::ModInfoArgs;
use crate::commands::arg_structs::install_args::InstallArgs;
use crate::commands::arg_structs::list_args::ListArgs;
use crate::commands::arg_structs::modpack_args::ModpackCommands;
use crate::commands::arg_structs::search_args::SearchArgs;
use crate::commands::arg_structs::sync_args::SyncArgs;
use crate::commands::arg_structs::update_args::UpdateArgs;
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use crate::commands::arg_structs::delete_args::DeleteArgs;
use crate::commands::arg_structs::download_args::DownloadArgs;
use crate::commands::arg_structs::updater_args::UpdaterArgs;

#[derive(Parser)]
#[command(name = "Rustique")]
#[command(author = "Theysa")]
#[command(about = "An extremely fast mod manager for Vintage Story, written in Rust.")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {

    /// Shows info level logging messages. This is very noisy, used for debugging.
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// Shows all logging messages. This is EXTREMELY noisy. Only run this if you have to.
    #[arg(short, long, default_value = "false")]
    pub debug: bool,

    /// Specify the directory to manage mods. This takes priority over any other directory setting, including from the config file.
    #[arg(short, long)]
    pub mods_dir: Option<String>,
    
    /// This command will set the working mod directory to be that of the modpack specified, INCLUDING modpacks you create. 
    /// If you use this to work on a custom modpack, you will need to run Rustique modpack create again to update your modpack file, just set the --mpk-id to the same one you used before to overwrite the old one.
    #[arg(short, long)]
    pub with_mpk: Option<String>,
    
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Checks with the VintageStory mods website for any updates to mods you have installed. Run update after this command to update your mods")]
    Sync(SyncArgs),

    #[command(about = "List installed mods and their versions and any missing dependencies. Running sync first will show any available updates to your mods")]
    List(ListArgs),

    #[command(about = "Updates a specific mod OR all mods installed. Runs sync after completion")]
    Update(UpdateArgs),

    #[command(about = "Install a specific mod. Must use the mod_id, Example: ./Rustique install alchemy")]
    Install(InstallArgs),
    
    #[command(about = "Search the mod website for new mods, Example: ./Rustique search -q magic")]
    Search(SearchArgs),
    
    #[command(about = "Manage config options for Rustique")]
    Config(ConfigCommand),

    #[command(about = "Miscellaneous items for Rustique, like shell auto-completion")]
    Misc {
          #[arg(short, long = "gen-auto-complete", value_name = "SHELL")]
          gen_auto_complete: Option<ShellType>,
    },
    
    #[command(about = "Download a Vintage Story executable")]
    Download(DownloadArgs),

    #[command(about = "Get more information about the mod specified")]
    Info(ModInfoArgs),
    
    #[command(about = "Create, download, update modpacks for VintageStory")]
    Modpack(ModpackCommands),
    
    #[command(name = "self", about = "Manage the Rustique binary; Check for updates, perform updates.")]
    RustiqueSelf(UpdaterArgs),
    
    #[command(about = "Remove mods and backups")]
    Delete(DeleteArgs)
}



#[derive(Args, Debug)]
pub struct ModIDSync {
   pub force: bool,
}

#[derive(Clone, ValueEnum)]
pub enum ShellType {
    Bash,
    Fish,
    Zsh,
    PowerShell
}

impl From<ShellType> for Shell {
    fn from(shell: ShellType) -> Self {
        match shell {
            ShellType::Bash         => Shell::Bash,
            ShellType::Fish         => Shell::Fish,
            ShellType::Zsh          => Shell::Zsh,
            ShellType::PowerShell   => Shell::PowerShell,
        }
    }
}
