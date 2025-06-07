use clap::Args;
use core::aliases::ModID;

#[derive(Args, Debug, Clone)]
pub struct ModInfoArgs {
   
    
    #[arg(num_args = 1..)]
    pub mod_id: Vec<ModID>,
   
    /// Shows the description of the mod, Note: This can take a lot of space in the terminal
    #[arg(short = 'd', long)]
    pub show_description: bool,
    
    /// Shows the last NUM amount of versions. Use with 0 to see ALL versions.
    #[arg(short = 'v', long, default_value = "3", value_name = "NUM")]
    pub show_versions: usize,
}
