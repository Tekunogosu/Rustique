# Rustique

Rustique is a command-line interface for managing and updating Vintage Story mods.


## How Rustique Works

Rustique works like many Operating System package managers such as Portage and Apt: first you sync a local copy of the package information, and then you can install, remove, update, and delete mods. Dependencies are also handled automatically.


## Commands

The commands are as follows:

| Command | Explanation |
|---------|-------------|
| `./Rustique -h` | Get help - list all the commands available  |
| `./Rustique sync` | Check for new versions of all the currently installed mods, then save the resulting metadata to the sync file |
| `./Rustique list` | List the currently installed mods, and their current updated versions available according to the sync file. Make sure you sync before list! |
| `./Rustique list --updates` | Same as list but only outputs lines where the current installed version doesn't match the newest available version |
| `./Rustique --mods-dir ~/vintage_story/Mods sync` | Specify the directory to operate on instead of the default. The default is automatically determined by operating system, but this can be used to easily manage multiple Vintage Story servers with different sets of mods |
| `./Rustique update --all` | Update all mods that have available updates |
| `./Rustique update primitivesurvival goblinears` | Update multiple packages by name. These are case-insensitive |
| `./Rustique install alchemy` | Install a new mod and its dependencies. Case-insensitive |
