use clap::{ArgGroup, Args};

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
   
    #[cfg(unix)]
    /// This is only used with the 1-click installer to wait for a keypress to keep the terminal open during installation. 
    #[arg(short, default_value = "false")]
    pub wait: bool,
}