use clap::{Args};
use crate::commands::search::{Field, SortBy, SortOrder};
use crate::config_structs::CellColor;

#[derive(Args)]
pub struct SearchArgs {

    /// This text query will search all available text fields, author, summary, name, urlalias, tags, side, type
    #[arg(short, long)]
    pub query: Option<String>,

    #[arg(short, long, )]
    pub color: Option<CellColor>,


    /// If you know the field name from the api you can search on it directly.
    ///
    /// This argument must be used with --query or nothing happens
    ///
    ///
    #[arg(short, long, requires = "query")]
    pub field: Option<Field>,

    /// Search by specific author.
    /// Note that the API doesn't appear to include more than 1 author, so this works for just the main author
    #[arg(short,long)]
    pub author: Option<String>,




    /// Search by a specific type of tag, example: Weapons or Technology
    #[arg(short, long)]
    pub tag: Option<String>,

    #[arg(short = 's', long)]
    pub sort_by: Option<SortBy>,

    #[arg(short = 'd', long)]
    pub sort_direction: Option<SortOrder>,
}
