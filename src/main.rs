#![allow(unused_imports, dead_code)]

mod sync;
mod list;
mod update;
mod changelog;
mod install;
mod utils;
mod api_structs;
mod api;

use std::path::PathBuf;
use clap::{Args, Parser, Subcommand, ColorChoice, CommandFactory, FromArgMatches, crate_authors};
use colored::Colorize;
use crate::utils::{get_expanded_path, RustiqueOptions};
use crate::list::list_installed;
use crate::sync::sync;
use crate::update::update;
/*

./vsupdate
To list all packages, run `./vsupdate list`
To sync the local package index, run `./vsupdate sync`
To update all packages, run `./vsupdate update --all`
To specify an alternative Mod directory, use `./vsupdate --mod-dir /path/to/Mods ..your command..`
To see the changelog for a package, run `./vsupdate changelog primitivesurvival`


# sync local package index
./vsupdate sync

# list currently installed mods
./vsupdate list
Local package index last updated 2025-04-18 05:30:00 PM
+-------------------+-------+---------+
| Mod               | Yours | Current |
+===================+=======+=========+
| primitivesurvival | 3.7.4 | 3.7.5   |
| goblinears        | 2.1.0 | 2.1.1   |
+-------------------+-------+---------+

# specify Vintage Story directory (housing Mods/ folder) instead of default, usually single-player client folder
./vsupdate --vs-dir ~/Downloads/vintagestory/vs_client_linux-x64_1.20.7/vintagestory list

# specify Mods directory directly e.g. for uncommon setups where administrator manages mods separately from server folder
./vsupdate --mods-dir ~/vintage_story/Mods list

# update all
./vsupdate update --all
primitivesurvival updated from 3.7.4 to 3.7.5
goblinears updated from 2.1.0 2.1.1

# run update when already up-to-date according to local package index
./vsupdate update --all
Nothing to update. Did you forget to `./vsupdate sync`?
Local package index last updated 2025-04-18 05:30:00 PM

# update one package
./vsupdate list
+-------------------+-------+---------+
| Mod               | Yours | Current |
+===================+=======+=========+
| primitivesurvival | 3.7.4 | 3.7.5   |
| goblinears        | 2.1.0 | 2.1.1   |
+-------------------+-------+---------+
./vsupdate update primitivesurvival
primitivesurvival updated from 3.7.4 to 3.7.5

# print the author's changelog for all versions from your currently installed to the current. or, if you already have the most recent installed already, print the changelog for the current version.
./vsupdate changelog primitivesurvival


Locations:

* ~/.config/VintagestoryData/ModsByServer/192.168.1.228-42420/primitivesurvival_3.7.5.zip
* vintagestory/Mods

 */

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long)]
    mods_dir: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Sync(SyncArgs),
    List(ListArgs),
    Update(UpdateArgs),
    Changelog(ChangeLogArgs),
    Install(InstallArgs),
    Info(ModInfoArgs),
    Search(SearchMods),
}


#[derive(Args)]
struct SyncArgs {
}

#[derive(Args)]
struct ListArgs {
}

#[derive(Args)]
struct UpdateArgs {
    name: Vec<String>,
    #[arg(short, long)]
    all: bool,
}

#[derive(Args)]
struct ChangeLogArgs {
    name: Option<String>,
}

#[derive(Args)]
struct InstallArgs {
    mod_id: Vec<String>,
}

#[derive(Args)]
struct ModInfoArgs {
    mod_id: String,
}

#[derive(Args)]
struct SearchMods {

}



fn list() {
    println!("+-------------------+-------+---------+\n\
| Mod               | Yours | Current |\n\
+===================+=======+=========+\n\
| primitivesurvival | 3.7.4 | 3.7.5   |\n\
| goblinears        | 2.1.0 | 2.1.1   |\n\
+-------------------+-------+---------+");
}

// TODO: Add feature to notify user when the modinfo.json file is malformed


fn main() {

    let cli = Cli::parse();
    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd


    let mod_opts = if cli.mods_dir.is_none() {
        RustiqueOptions::default()
    } else {
        RustiqueOptions {
            mod_dir: Some(get_expanded_path(PathBuf::from(cli.mods_dir.unwrap()))),
            mod_id: None
        }
    };

    // TODO: check for windows equiv
    match &cli.command {

        // Database fields
        // modid
        // installed version
        // latest version
        // last sync time
        // url to latest known version

        Commands::Sync(_name) => {
            // Sync will add a rustique-sync.json to a valid mod_dir
            sync(mod_opts).unwrap()
        }
        Commands::List(_name) => {
            list_installed(mod_opts).unwrap();
        }
        Commands::Update(name) => {
            if name.all {
                update(mod_opts).unwrap();
            }
            else if name.name.is_empty() {
                println!("Must specify at least one package to update!");
            }
            else {
                println!("updating the following packages: {:?}", name.name);
            }
        }
        Commands::Changelog(name) => {
            println!("list {:?}", name.name);
        }
        Commands::Install(args) => {
            println!("install {:?}", args.mod_id);
        }
        Commands::Info(args) => {
            println!("displaying stuff about the mod {:?}", args.mod_id);
        }
        Commands::Search(_args )=> {
            print!("Searching stuff");
        }
    }
}