use clap::Args;

#[derive(Args)]
pub struct SyncArgs {
    /// Sync the mod ids from the api and save it to a local file in ~/.config/rustique/mod-id-sync.json.
    ///
    /// This is created to help Rustique manage mods which do not provide a `mod_id` in their `modinfo.json` file (which it's suppose to).
    ///
    /// Synced automatically once a day. Use with --force to update now.
    #[arg(short, long)]
    pub ids: bool,

    /// Used with --ids to force an update of the mod ids
    #[arg(short, long, requires = "ids")]
    pub force: bool,
}