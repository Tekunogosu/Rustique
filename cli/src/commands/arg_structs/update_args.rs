use clap::Args;

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