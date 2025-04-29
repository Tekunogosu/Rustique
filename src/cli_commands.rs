use clap::{Args, Parser, Subcommand, ArgGroup};
use crate::modpack_commands::*;
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {

    #[arg(short, long)]
    pub(crate) mods_dir: Option<String>,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Checks with the VintageStory mods website for any updates to mods you have installed. Run update after this command to update your mods")]
    Sync,

    #[command(about = "List installed mods and their versions and any missing dependencies. Running sync first will show any available updates to your mods")]
    List(ListArgs),

    #[command(about = "Updates a specific mod OR all mods installed. Runs sync after completion")]
    Update(UpdateArgs),

    #[command(about = "View the changelogs for a installed mod (Not Implemented)")]
    Changelog(ChangeLogArgs),

    #[command(about = "Install a specific mod. Must use the mod_id, Example: ./Rustique install alchemy")]
    Install(InstallArgs),

    #[command(about = "Shows values from the modinfo.json file inside the mod zip")]
    Info(ModInfoArgs),

    #[command(about = "Search the mob website for mobs. (Not implemented)")]
    Search(SearchMods),

    #[command(about = "Work in progress")]
    ModPack {
        #[clap(subcommand)]
        command: ModpackCommands,
    },
}

// #[derive(Args)]
// pub struct SyncArgs {
// }

#[derive(Args)]
pub struct ListArgs {
    /// List only mods that need updating
    #[arg(short, long, default_value = "false")]
    pub(crate) updates: bool
}

#[derive(Args)]
pub struct UpdateArgs {

    /// Update specific mod, must be mod_id. Example: ./Rustique update alchemy
    #[arg(num_args = 1..)]
    pub(crate) mod_ids: Vec<String>,

    /// Update all mods, don't set a <name>. Example: ./Rustique update --all
    #[arg(short, long)]
    pub(crate) all: bool,

    /// Update mods but keep old version.
    #[arg(short, long, default_value = "false")]
    pub(crate) keep_old_files: bool
}

#[derive(Args)]
pub struct ChangeLogArgs {
    pub(crate) name: Option<String>,
}



#[derive(Args)]
#[command(group(
    ArgGroup::new("dependency_flags")
        .args(["missing_dependencies", "ignore_dependencies"])
        .required(false)
))]
pub struct InstallArgs {
    /// List all the mods you want to install with a space between each mod. Example ./Rustique install alchemy combatoverhaul
    #[arg(num_args = 1..)]
    pub(crate) mod_ids: Vec<String>,

    /// Setting this flag will prevent Rustique from installing dependencies discovered during installation
    #[arg(short, long, default_value = "false")]
    pub(crate) ignore_dependencies: bool,

    /// This flag will install all missing dependencies found within your mod directory
    #[arg(short, long, default_value = "false")]
    pub(crate) missing_dependencies: bool,
}

#[derive(Args)]
pub struct ModInfoArgs {
    pub(crate) mod_id: String,
}

#[derive(Args)]
pub struct SearchMods {

}
