use clap::{ArgGroup, ValueEnum};
use clap::{Args, Subcommand};
use crate::config_structs::{CellAttr, CellColor, ListColumn, SearchColumn};

#[derive(Args)]
pub struct TableArgs {
    #[command(subcommand)]
    pub subcommand: TableSubCommands
}

#[derive(Subcommand)]
pub enum TableSubCommands {
    /// Modify which columns should be present and set their color and attributes
    Set(TableSetArgs),

    /// Remove a column from the table:
    ///
    /// Note: This removes the column from the config, you will need to use `set` to add it back
    ///
    Del(TableSetArgs),

    /// Shows the current values defined in the config file
    List,
}

#[derive(Args)]
pub struct TableSetArgs {
    #[command(subcommand)]
    pub subcommand: TableArgsSubCommands
}

#[derive(Subcommand)]
pub enum TableArgsSubCommands {
    ///
    /// Modify the List commands display table
    ///
    /// Example
    ///
    /// Show only name, mod-id, and version and set all of them to green with bold text
    ///
    /// ./Rustique config table set list --headers --fields name,mod_id,version --color green --attr bold
    ///
    List(TableSubFlags<ListColumn>),

    ///
    /// Modify the Search commands display table
    ///
    /// Example
    ///
    /// Show only name, mod-id, and last-released with blue and bold text
    ///
    /// ./Rustique config table set search --headers --fields name,mod-id,last-released --color blue --attr bold
    ///
    Search(TableSubFlags<SearchColumn>),
}

#[derive(Args)]
#[command(group(
    ArgGroup::new("field_or_fields")
    .args(["field", "fields"])
    .multiple(false)
    .required(true)
))]
pub struct TableSubFlags<T>
where T: ValueEnum + Clone + Send + Sync + 'static {

    #[clap(flatten)]
    pub group: TableGroup,

    ///
    /// Field lets you modify 1 cell at a time, this gives you the most granular configuration.
    ///
    /// You must specify at least `1` field
    ///
    #[arg(short, long, requires = "table_flags", value_name = "FIELD")]
    pub field: Option<T>,

    ///
    /// Fields lets you modify many fields at the same time. If you use this with --color or --attr
    /// you will set ALL provided fields to those colors and with those attributes
    ///
    #[arg(short = 'F', long, requires = "table_flags", value_name = "FIELDS", num_args = 1..)]
    pub fields: Vec<T>,

    ///
    /// Set the cell color
    ///
    #[arg(short = 'r',long, requires = "table_flags", requires = "fields", value_name = "COLOR")]
    pub color: Option<CellColor>,

    ///
    /// Set the attribute of the cell. For now you can only specify `1` attribute at a time
    ///
    #[arg(short, long, requires = "table_flags", requires = "fields",value_name = "ATTR")]
    pub attr: Option<CellAttr>,
    
    #[arg(long)]
    pub reset: bool,
}


#[derive(Args)]
#[command(group(
    ArgGroup::new("table_flags")
        .args(["headers", "cells"])
        .multiple(false)
        .required(true)
))]
pub struct TableGroup {
    /// Use this flag to modify the table headers
    #[arg(short = 'H',long = "headers")]
    pub headers: bool,

    /// Use this flag to modify the table cells in the body of the table
    #[arg(short = 'C', long = "cells")]
    pub cells: bool,
}
