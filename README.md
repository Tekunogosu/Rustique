
# Rustique

Rustique is a command-line interface, written in Rust, for managing and updating Vintage Story mods and their dependencies. 

_...and its fast af!_


Rustique is currently in Alpha, meaning many features are not present. In its current state, it can easily manage all your mods and their dependencies without issue. 

### Want to support Rustique? 

Any donation is appreciated! Bug reports and pull requests are also good! There is no obligation here, so support only if you can or want!

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/O5O13Y88O)

## How Rustique Works

Rustique works like many Operating System package managers such as Portage and Apt: first you `sync` a local copy of the package information, and then you can `install`, `list`, and `update` mods. Dependencies are handled automatically! 

### Simple update for all mods
##### _This is the most basic example of how to update your mods._ 

`./Rustique sync` -- Create a local file with update information about your installed mods.

`./Rustique list --updates` OR `./Rustique list -u` -- Optional: This command will tell you which mods need to be updated, including any missing dependencies for those updates.

`./Rustique update --all` OR `./Rustique update -a` -- Updates all installed mods. This will also download missing dependencies for the mods that actually need updating. 
 

### Install all missing dependencies
##### _We've all downloaded mods that have dependencies which were not stated on the mod site.. which is frustrating. Here's how you deal with that._

`./Rustique install --missing-dependencies`
OR
`./Rustique install -m`

This command will install ALL missing dependencies found for any installed mods. This is a recursive check, which means any dependencies installed will also be checked for dependencies.


### Installing new mods
This requires you to have the mod id in order to use the `install` command. For _most_ mods you can obtain this by looking at the URL bar on the mods website. _(Soon the search function will be implemented for Rustique)_.

**Example:** `https://mods.vintagestory.at/alchemy` The mod id here is `alchemy`. Some mods will not have this value that you can easily obtain, so you'll have to download those manually. But don't worry, Rustique will manage those manually downloaded without any issue.

Here's an example of a URL without the mod id: `https://mods.vintagestory.at/show/mod/21737` -- 21737 is actually an asset ID and cannot be used with the API. This mod would require you to manually download it.


`./Rustique install alchemy armory combatoverhaul` -- Downloads all 3 mods listed and their dependencies. Any invalid mod id will show an error message, the rest will continue to download.


## Commands

The commands are as follows:

| Command                                           | Explanation                                                                                                                                                                                                               |
|---------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `./Rustique help`                                 | Get help - list all the commands available                                                                                                                                                                                |
| `./Rustique help sync`                            | Show more info about the command specified. Replace `sync` with any of the other commands                                                                                                                                 |
| `./Rustique sync`                                 | Check for new versions of all the currently installed mods, then save the resulting metadata to the sync file                                                                                                             |
| `./Rustique list`                                 | List the currently installed mods, and their current updated versions available according to the sync file. Make sure you sync before list!                                                                               |
| `./Rustique list --updates`                       | Same as list but only outputs lines where the current installed version doesn't match the newest available version                                                                                                        |
| `./Rustique --mods-dir ~/vintage_story/Mods sync` | Specify the directory to operate on instead of the default. The default is automatically determined by operating system, but this can be used to easily manage multiple Vintage Story servers with different sets of mods |
| `./Rustique update --all`                         | Update all mods that have available updates                                                                                                                                                                               |
| `./Rustique update primitivesurvival goblinears`  | Update multiple packages by name. These are case-insensitive                                                                                                                                                              |
| `./Rustique install alchemy`                      | Install a new mod and its dependencies. Case-insensitive                                                                                                                                                                  |
| `./Rustique install --missing-dependencies`       | Install all missing dependencies found in your mod directory                                                                                                                                                              |
| `./Rustique config set --mod-dir /path/to/mods`   | Set your default mod directory if non-default. This way you don't have to pass -m each time you use Rustique                                                                                                              |
| `./Rustique info --mod-id alchemy`                | See more information, including changelogs and versions, for the specified mod. Use with [-v num] to see `num` amount of changelogs                                                                                       |
| `./Rustique search -q magic`                      | Search the mods DB for any mod that has the word magic in it. -q by itself is a generic search and searches all text fields. See `Rustique help search` for more options.                                                 |


## Missing Features
* `Modpack` creation, installation, and management. There is no default mod pack capability for VS, so Rustique will fill that void.
* GUI - Yes, a GUI is planned! This is a ways out and will only happen after Rustique is more or less feature complete. 


## Known Issues
* **None:** If you find any, please open an issue here on Github. Seriously, I need you to break it!