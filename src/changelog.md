# Version 0.3.1-alpha


# Version 0.3.0-alpha
* Implemented search! Checkout `Rustique help search` on how to use it
* You can now change what information to display with search via the config file!
* You can modify the display table for the list and search command via Rustique! Checkout ./Rustique config help table. A wiki page will be going up for this as there is a lot to it.
* If the config.toml file is malformed, the malformed config is backed up and a new config will be created with a nice message.

# Version 0.2.6-alpha
* Fixed missing mod id for mods that have malformed modinfo.json file but rustique was able to obtain the numerical mod id.

# Version 0.2.5-alpha
* Fixed display issue where dependencies were being duplicated when viewing `list`
* Fixed issue where the config folder for rustique was being created in the wrong place on linux. 

# Version 0.2.4-alpha
* You can now reset config values with `Rustique config del [OPT]`, see `Rustique config help del`
* You can now list all config values with `Rustique config list`

# Version 0.2.3-alpha
* Fixed invalid missing dependencies showing up when you type list -u

# Version 0.2.2-alpha
* Fixed version parsing error for versions that show up as NULL from the api. This was just a display issue, didn't actually affect the usage of Rustique

# Version 0.2.1-alpha
* Using -v now shows the correct mod directory that Rustique is looking at
* Added compiler flags to make the Rustique binary smaller and reduced features from used libraries. (saves a few MBs from the executable)
* Added misc command for generating auto complete for the shells zsh,bash,fish,powershell. `./Rustique help misc`

# Version 0.2.0-alpha
* Fixed api error message with blank info. 
* Config file is now live! You can easily set the default mod directory so you don't have to use -m for each use if different than default.
* To manage the config file you use Rustique directly. Checkout `Rustique help config` for all options. Note that not everything is implemented yet.
* Reorganized code base a bit, this doesn't affect anything user side, but it's a win for me. :3
* Added logging lib and implemented --verbose and --debug. --verbose will show some extra messages if you notice some problems. --debug you should only use if told to do so, its extremely noisy and floods the terminal.
* The description from the mod files is now sanitized to strip any newline or tab characters as it messes with the `list` table formatting. If you find any other mods that don't show up correctly, please report it.
* Fixed some versioning bugs when using sync and update that would cause some mods to not be updated. 
* Added an operation time footer for `list`, `update`, `sync`, and `install`. This can be turned off in the configs.
* List shows total mods installed at the bottom. For now, this only shows the valid mods that Rustique can actually manage. Any non-zip mods are ones that list can't read, will not be counted. 
* The list table style is slightly more compact now and the dependencies lists no longer wrap in the middle of a long mod_id.
* Adjusted the look'n feel of the tables.
* Information text has a border now.
* Rustique no longer deletes mods that are malformed during the update command, it reports the problem but leaves it alone.
* Full rework of mod installation and dependency resolution. Update & Install are dramatically faster.
* Reworked how the modinfo.json data is extracted, should make sync a bit faster.
* Rustique can now attempt to find the modid from various methods to manage your mods. Users will be notified if Rustique cannot determine the mod_id. 
* After installing updates or new mods, there is now a table that displays the mods and their versions. 