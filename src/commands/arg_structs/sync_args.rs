use clap::Args;

#[derive(Args)]
pub struct SyncArgs {
    /// Sync the mod info from the api endpoint /api/mods and save it locally.
    /// 
    /// This file is used for the search command so we are not making a ton of unneeded api calls and
    /// to obtain the mod-id for mods that don't provide one in their modinfo.json
    /// 
    /// linux/mac: [~/.config/rustique/mod-id-sync.json.]
    /// 
    /// windows: [%appdata%/rustique/mod-id-sync.json]
    ///
    #[arg(short = 's', long)]
    pub sync_search_db: bool,
    
    /// Sync a local list of all game versions from the api. This is used with version pinning to ensure
    /// we have accurate version numbers.
    /// 
    /// linux/mac: [~/.config/rustique/game-versions.json]
    /// 
    /// windows: [%appdata%/rustique/game-versions.json]
    /// 
    #[arg(short = 'g', long)] 
    pub sync_game_versions: bool,
}