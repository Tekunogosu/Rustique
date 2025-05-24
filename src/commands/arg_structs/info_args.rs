use clap::Args;
use crate::aliases::ModID;

#[derive(Args, Debug, Clone)]
pub struct ModInfoArgs {
    
    #[arg(short, long, value_name = "MODID")]
    pub mod_id: ModID,
   
    /// Shows the description of the mod, Note: This can take a lot of space in the terminal
    #[arg(short = 'd', long)]
    pub show_description: bool,
    
    /// Shows the last NUM amount of versions. Use with 0 to see ALL versions.
    #[arg(short = 'v', long, default_value = "3", value_name = "NUM")]
    pub show_versions: usize,
}
