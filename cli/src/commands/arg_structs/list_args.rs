use std::path::PathBuf;
use clap::{Args, ValueEnum};
use core::config::config_structs::ListColumn;

#[derive(Args, Debug, Clone)]
pub struct ListArgs {
    /// List only mods that need updating
    #[arg(short, long, default_value = "false")]
    pub updates: bool,

    /// (Does not work with modpack commands) List all game versions for MAJOR.MINOR: Example, Rustique list --game-versions 1.20, which will show all valid versions for 1.20.x, --game-versions 1 will show all versions 1.x.x
    #[arg(short, long, value_name = "MAJOR.MINOR")]
    pub game_versions: Option<String>,
   
    #[clap(flatten)]
    pub export_args: ListOutputArgs
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ListExport {
    Csv,
}

#[derive(Args, Debug, Clone)]
pub struct ListOutputArgs {

    /// Instead of printing the text table, export to this type instead. IF you use csv AND show the changelog column, you may not be able to redirect to a file as long text can get truncated. Use -f /path/to/save.csv instead.
    #[arg(short, long)]
    pub export_as: Option<ListExport>,

    /// Show specific columns for the list output. This flag takes priority over the config values for the list display.
    #[arg(short, long, num_args = 1.., value_name = "COLUMNS")]
    pub columns: Vec<ListColumn>,

    /// Set this to save output to file INSTEAD of stdout, used with --export-as
    #[arg(short, long, value_name = "PATH", requires = "export_as")]
    pub file_path: Option<PathBuf>,
}