use clap::Args;
use crate::aliases::ModID;

#[derive(Args)]
pub struct ModInfoArgs {
    
    #[arg(short, long, value_name = "MODID")]
    pub mod_id: ModID,
    
    #[arg(short, long)]
    pub show_versions: bool,
    
    #[arg(short, long, requires = "show_versions")]
    pub last: Option<u32>,
    
    
}
