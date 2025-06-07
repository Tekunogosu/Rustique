# Version 0.5.5-alpha
* **WINDOWS ONLY:** Changed the default path for Windows to `%appdata%/VintagestoryData/Mods`. Originally it was `%appdata%/Vintagestory/Mods`, which is technically works, but the preferred location is VintagestoryData. I was looking at an old wiki page when getting this information initially.
* **WINDOWS ONLY:** Because of the change to the default mod path for windows, Rustique will now ask if you want to move the mods when you first run Rustique. This will not affect your game in any way, just where the mods get loaded from.
* Added new flags to `list`, `--columns`: Lets you show only specific columns, `--export`: Change the output format, currently only for csv, `--file-path`: save the output to a file instead of printing to stdout. 

# Version 0.5.4-alpha
* Added progress bars for mod api fetches and downloading of mod files. Will possible add more in other sections at a later time.
* Added new flag; `Rustique modpack install --missing-dependencies mpkid`. This helps download mods for a modpack that you install manually. 

# Version 0.5.3-alpha
* Fixed regression bug where sync would not lowercase the modid after making the API calls for the mods. This caused the sync file to create a new entry that didn't have the required information to update these types of mods.

# Version 0.5.2-alpha
* Fixed invalid type error with api call when checking for update for rustique. 

# Version 0.5.1-alpha
* Fixed bug with delete that was making the command pretty slow.

# Version 0.5.0-alpha - Self Updating.. Update!
* Rustique now has a self update!! Rustique will update in place. Check out `Rustique help self` to see the commands. (You can check for updates and perform an update)
* Fixed `list --updates` showing all mods instead of only ones that need updates. This fixes issue #12.
* `list` now runs sync automatically if the sync file is missing from an installed modpack. This raised an error before.
* Added `--with-mpk` to the base `Rustique` flags as a shortcut to handling modpack mods. You'll be able to use the base commands, `list`,`update`,`install`, on the packpack mods dir.  `Rustique -w tmmv update -a` -- this will update all the individual mods in the tmmv modpack.
* You can now install a mod of a specific version with the `install` command, use mod-id@version. Example `Rustique install alchemy@1.6.50`. 
* Mod backups have been implemented! It's disabled by default, you can turn it on with `Rustique config set --backup-mods true`. You can also choose where they are stored with `Rustique config set --backup-mods-dir /path/to/dir`
* You can now `delete` mods! Checkout `Rustique help delete`

# Version 0.4.2-alpha
* Fixed regression bug #11 that fails to decode api json data when the file_id for a release is null

# Version 0.4.1-alpha
* Fully switched to the tokio-async library, swapped from synchronous zip lib to async-zip. This shows a slight increase in performance when using Rustique on a lot of mods at once.
* `modpack create` now sets your own mods up in a way that will let you enable/disable them. 
* Added `modpack local list` to view your locally created modpacks. Enable/disable work like normal. 
* A flag for `modpack create --ignore-modpacks` was added to let you choose to ignore enabled modpacks. Its set to false by default so you can create new modpacks out of existing ones.
* A flag for `modpack create --copy-mods` was added for choosing to copy the mods made by the command instead of moving them. By default when you create a modpack, the mods are moved into the ~/.config/rustique/modpacks/installed/yourpack folder. If you set --copy-mods, the orignal mods will stay in place and a copy will be created into the installed dir.
* All integer api_arg values are now i64 instead of u32. The api changed how game versions are handled and now show a large negative i64 value. 

# Version 0.4.0-alpha
* Implemented modpack functions! Create, Install, Enable/Disable, List, Info
* The modpack functionality has a lot to it. See the wiki for usage examples
* There are now builds for Intel and Arm based Macs! I used Github Actions, so it *should* work. Its untested though..
* Various bug fixes and tweaks

# Version 0.3.2-alpha
* Implemented the `Rustique download` command to download versions of the Vintage Story game itself. You can specify where its saved via the command line or the config. Default save location is your Downloads folder.
* Implemented `Rustique list --game-versions` which will show all valid game versions. This will show valid versions for the new `download` command and version pinning, which was implemented last update.

# Version 0.3.1-alpha
* Version pinning is live! You can now pin a specific mod version and/or a specific game version. There are some situations where you get weird results by setting both or mods not having an game version that matches at all. Please report any situations where you encounter errors. 
* You can now enable the following columns for the list display table; Pinned Version, Game Version.
* To pin a specific version of a mod use `Rustique config set -w mod-id -P 1.2.3`
* To pin a specific game version use `Rustique config set -p 1.20.8`
* The "Latest Version" in list command will now show (pinned) if a version has been pinned.
* Columns with numbers displayed should now be right aligned.
* The Info command is now live. See `Rustique help info ` to see how it use it.

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