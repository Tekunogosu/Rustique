use clap::{Args, Parser, Subcommand, ArgGroup, ValueEnum};
use clap_complete::Shell;
use crate::commands::arg_structs::changelog_args::ChangeLogArgs;
use crate::commands::arg_structs::config_args::ConfigCommand;
use crate::commands::arg_structs::info_args::ModInfoArgs;
use crate::commands::arg_structs::install_args::InstallArgs;
use crate::commands::arg_structs::list_args::ListArgs;
use crate::commands::arg_structs::modpack_args::ModpackCommands;
use crate::commands::arg_structs::search_args::SearchMods;
use crate::commands::arg_structs::sync_args::SyncArgs;
use crate::commands::arg_structs::update_args::UpdateArgs;

#[derive(Parser)]
#[command(name = "Rustique")]
#[command(author = "Theysa")]
#[command(about = "An extremely fast mod manager for Vintage Story, written in Rust.")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {

    /// Shows info level logging messages. This is very noisy, used for debugging.
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// Shows all logging messages. This is EXTREMELY noisy. Only run this if you have to.
    #[arg(short, long, default_value = "false")]
    pub debug: bool,

    #[arg(short, long)]
    pub(crate) mods_dir: Option<String>,

    #[command(subcommand)]
    pub(crate) command: Commands,
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

    #[command(about = "Manage config options for Rustique")]
    Config(ConfigCommand),

    #[command(about = "Miscellaneous items for Rustique, like shell auto-completion")]
    Misc {
          #[arg(short, long = "gen-auto-complete", value_name = "SHELL")]
          gen_auto_complete: Option<ShellType>,
    },

    #[command(about = "View the changelogs for a installed mod (Not Implemented)")]
    Changelog(ChangeLogArgs),

    #[command(about = "Shows values from the modinfo.json file inside the mod zip (Not Implemented)")]
    Info(ModInfoArgs),

    #[command(about = "Search the mod website for mods. (Not implemented)")]
    Search(SearchMods),

    #[command(about = "Create, download, update modpacks for VintageStory (Not Implemented)")]
    ModPack {
        #[clap(subcommand)]
        command: ModpackCommands,
    },

    #[cfg(feature = "dev")]
    #[command(about = "Bulk downloader")]
    BulkDownloader(BulkDownloadCommands),

    #[cfg(feature = "dev")]
    #[command(about = "test command")]
    TestCommand(TestArg),

    #[cfg(feature = "dev")]
    #[command(about = "load mods from a text file")]
    LoadMods(LoadModsArgs)

}

#[derive(Args, Debug)]
pub struct ModIDSync {
   pub force: bool,
}

#[cfg(feature = "dev")]
#[derive(Args, Debug)]
pub struct LoadModsArgs {
    #[arg(short, long)]
    pub filename: String,
}

#[cfg(feature = "dev")]
#[derive(Args)]
pub struct TestArg {
    #[arg(short, long)]
    pub version_to_pin: String,
}

#[cfg(feature = "dev")]
#[derive(Args)]
pub struct BulkDownloadCommands {
    /// Number of mods to download
    #[arg(short, long, default_value = "100")]
    pub(crate) num_to_download: usize,
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
            ShellType::Bash => Shell::Bash,
            ShellType::Fish => Shell::Fish,
            ShellType::Zsh => Shell::Zsh,
            ShellType::PowerShell => Shell::PowerShell,
        }
    }
}
