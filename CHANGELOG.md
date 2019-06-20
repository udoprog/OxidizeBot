# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
[Unreleased]: https://github.com/udoprog/setmod/compare/0.4.1...master

## [0.4.1]

### Changed
- UI doesn't break on bad semver latest releases.

[0.4.1]: https://github.com/udoprog/setmod/compare/0.4.0...0.4.1

## [0.4.0]

### Changed
- Don't crash if a user removes a the Twitch connection.
- Deprecate use of configuration file fully.

[0.4.0]: https://github.com/udoprog/setmod/compare/0.3.8...0.4.0

## [0.3.8]

### Added
- Added `!poll` command to run polls.
- Added `!weather` command to check the current weather.
- Notification after stream has ended if you have afterstream messages.

[0.3.8]: https://github.com/udoprog/setmod/compare/0.3.7...0.3.8

## [0.3.7]

### Changed
- `utils::compact_duration` now includes `days` when formatting.
- Don't use `flatMap` in since it's not supported in OBS's Browser Source.

[0.3.7]: https://github.com/udoprog/setmod/compare/0.3.6...0.3.7

## [0.3.6]

### Added
- `time/timezone` for setting your current time zone.
- `!time` command for showing the current time of the streamer (in the specified time zone) [#45].
- `system/run-on-startup` setting that can configure SetMod to run on system startup.

[#45]: https://github.com/udoprog/setmod/issues/45
[0.3.6]: https://github.com/udoprog/setmod/compare/0.3.5...0.3.6

## [0.3.5]

### Added
- `!speedrun game <game>` as a preferred alias for `!speedrun record`.
- `!speedrun personal-bests <user>` as a way to query personal bests for a single user.
- Commands can now use the `{{rest}}` parameter to expand to anything that comes after the command itself.
- Introduced the `irc/viewer-reward/interval` setting to tweak how frequently viewer rewards are posted.

### Changed
- Move most authorization checks to their own scopes (See [auth.yaml] for details) [#48].
  * `song/theme`
  * `song/edit-queue`
  * `song/list-limit`
  * `song/volume`
  * `song/playback-control`
  * `command/edit`
  * `theme/edit`
  * `promo/edit`
  * `alias/edit`
  * `countdown`
  * `gtav/raw`
  * `water/undo`
- Support temporary grants through `!auth permit 5m <user> <scope>`.
  You can only grants scopes that you have [#47].
  * For example, to permit posting any links: `!auth permit 30s setbactesting chat/bypass-url-whitelist`
- Fix issue where player feedback shuts down first time it's disabled.

[auth.yaml]: bot/src/auth.yaml
[#48]: https://github.com/udoprog/setmod/issues/48
[#47]: https://github.com/udoprog/setmod/issues/47
[0.3.5]: https://github.com/udoprog/setmod/compare/0.3.4...0.3.5

## [0.3.4]

### Added
- Notifications for system integration.
- Added `--log-config` and `--trace` switches to give more control over logging.

### Changed
- Fixed player sync on startup not setting state correctly.
- Fixed issue where Spotify can't be controlled unless it is started before the bot.

[0.3.4]: https://github.com/udoprog/setmod/compare/0.3.3...0.3.4

## [0.3.3]

### Changed
- Installer can now successfully stop SetMod during upgrades.
- Fixed issue where non-mods cannot request songs without stream currency enabled even though `song/*/min-currency` was set to `0`.
- `song/*/max-duration` is not optional and unset by default, and can be deleted.

[0.3.3]: https://github.com/udoprog/setmod/compare/0.3.2...0.3.3

## [0.3.2]

### Added
- Added update notification in web UI.

### Changed
- Fixed issue when picking device and simplified how device ID is stored.
- Fixed issue where delete dialog does not disappear.

[0.3.2]: https://github.com/udoprog/setmod/compare/0.3.1..0.3.2

## [0.3.1]

### Added
- Added a button to open the setmod log file in systray.
- Added a button to restart the bot from systray.

### Changed
- Fixed bug with syncing remote player state.
- Fixed bug with syncing player queue to setbac.tv.
- Reverted fix for Twitch API, since they fixed it themselves :|.
- Fixed broken link to player on setbac.tv.

[0.3.1]: https://github.com/udoprog/setmod/compare/0.3.0...0.3.1

## [0.3.0]

### Changed
- Changed where SetMod configuration is expected to be.
  See `Migrating from 0.2 to 0.3` in the README.

### Added
- Added an installer for SetMod which is recommended to use on Windows.

[0.2.10]: https://github.com/udoprog/setmod/compare/0.2.10...0.3.0

## [0.2.10]

### Changed
- Reduce the number of calls for `!speedrun` command.
- Commands can now be quoted to support spaces in arguments.
  * Example: `!speedrun game sm64 --category "120 Star"`

[0.2.10]: https://github.com/udoprog/setmod/compare/0.2.9...0.2.10

## [0.2.9]

### Changed
- Fixed bug where new tokens weren't stored locally properly.

[0.2.9]: https://github.com/udoprog/setmod/compare/0.2.8...0.2.9

## [0.2.8]

### Changed
- Themes are now stored in the database, accessible through the `!theme` command and the web UI.
- `[player]` has been deprecated in favor of `player` settings.
- `[current_song]` has been deprecated in favor of `player/song-file` settings.
- `[[modules]]` configuration has been deprecated in favor of their corresponding setting.
- Fully deprecated the need for a configuration file. If you want to migrate existing settings, run the bot once with the configuration file, then it can safely be deleted.
- Add more commands to `!gtav` and add command-specific overrides through `gtav/command-config`.

### Added
- Added the ability to scale the maximum volume of a player by a percentage using the following settings:
  * `player/spotify/volume-scale`
  * `player/youtube/volume-scale`
- Pinging and reconnect if connection to Twitch is lost.
- Tokens can now be removed, and refreshed on the home screen without restarting the bot.
- Authentication system with different scopes to control permissions. See [auth.yaml] for more details.
  * This includes the groups: **@streamer**, **@moderator**, **@subscriber**, and **@everyone**.
- A UI page to handle Authorization.
- Load fallback songs from `player/fallback-uri`.
- `!speedrun` command to get records from speedrun.com.

[auth.yaml]: bot/src/auth.yaml
[0.2.8]: https://github.com/udoprog/setmod/compare/0.2.7...0.2.8

## [0.2.7]

### Changed
- `!song request` no longer allows one extra request to enter the queue. ([#33])
- Avoid playing the same songs over and over ([#35]).
- Fix off-by-one check in currency transfer (`!currency give <user> <amount>`).
- Only non-moderator and non-streamer chat bumps the idle counter.
- URL whitelist is now stored in a setting `irc/whitelisted-hosts` ([#37]).
- Massively improved settings and schema management.
- Settings are now parsed from chat, meaning they are validated and doesn't have to be JSON.
- `!song request` can now search for YouTube videos through `!song request youtube:<query>`.

### Added
- `!admin version` to check current setmod-bot package version. ([#32])
- Setting for controlling player feedback in chat (`player/chat-feedback`).
- Setting for controlling overlay update interval (`player/song-update-interval`).
- `!admin settings` for reading and writing settings through chat.
- Introduced the setting `player/detached` to detach the player. ([#27])
- Confirmation response when performing `!song open` and `!song close`. ([#36])
- `!admin push <key> <value>` to insert values into settings which are collections.
- `!admin delete <key> <value>` to delete values from settings which are collections.
- Group management and the ability to enable and disable commands (`!command`), aliases (`!alias`) and promotions (`!promo`).
  * `!<thing> enable <name>` - Enable the given command.
  * `!<thing> disable <name>` - Disable the given command.
  * `!<thing> group <name>` - Get the current group.
  * `!<thing> group <name> <group>` - Set the current group.
  * `!<thing> clear-group <name>` - Remove from all groups.
  * `!admin enable-group <group>` - Enable all commands, promotions, and aliases belonging to the specified group.
  * `!admin disable-group <group>` - Disable all commands, promotions, and aliases belonging to the specified group.
- Experimental support for requesting YouTube songs and associated settings.
- `player/duplicate-duration` to enforce a minimum duration between requesting duplicates songs.
- Added `song/*/min-currency`, for a minimum currency limit to request songs.
- Added `song/*/subscriber-only` and `song/subscriber-only` to limit song requests to subscribers only.

[Unreleased]: https://github.com/udoprog/setmod/compare/0.2.6...HEAD
[#27]: https://github.com/udoprog/setmod/issues/27
[#32]: https://github.com/udoprog/setmod/issues/32
[#33]: https://github.com/udoprog/setmod/issues/33
[#35]: https://github.com/udoprog/setmod/issues/35
[#36]: https://github.com/udoprog/setmod/issues/36
[#37]: https://github.com/udoprog/setmod/issues/37

[0.2.7]: https://github.com/udoprog/setmod/compare/0.2.6...0.2.7

## [0.2.6]

### Changed
- `!song promote` now moves the promoted song to the front of the queue instead of swapping positions with the first song in the queue. ([#30])

### Added
- Import/Export for PhantomBot points.
- gtav: Add vehicle by name from https://bit.ly/gtavvehicles
- gtav: more commands

[0.2.6]: https://github.com/udoprog/setmod/compare/0.2.5...0.2.6
[#30]: https://github.com/udoprog/setmod/issues/30

## [0.2.5]

### Changed
- Expand `currency` feature. See the [`currency` configuration].
- Added `!thingies give` and `!thingies show`. See the [`currency` configuration].

### Added
- `!promo` now uses hangout detection determined by the `irc/idle-detection/threshold` setting.
- `!afterstream` command without argument now prints a help message ([#26]).
- Added `gtav` module to interface with [ChaosMod]. See [`gtav` configuration].
- Theme songs now can have an `end` parameter, indicating when it should end.
- Song requests can be rewarded using the `song/request-reward` setting.
- Water reward can be scaled using `water/reward%`.
- Viewer reward can be scaled using `irc/viewer-reward%`.

[`currency` configuration]: README.md#currency
[`gtav` configuration]: README.md#gtav
[ChaosMod]: https://github.com/udoprog/ChaosMod

[#26]: https://github.com/udoprog/setmod/issues/26

### Changed
- Fixed bug where `Settings` frontend would make the value into a string before sending it to backend.

[#26]: https://github.com/udoprog/setmod/issues/26

[0.2.5]: https://github.com/udoprog/setmod/compare/0.2.4...0.2.5

## [0.2.4]

### Added
- Web-based overlay with current song ([#22]).
- Player will no longer pause the current song (if it's playing) and will instead synchronize the state of the player with Spotify ([#18]).
- Implement `!command rename <from> <to>`
- Ability to sync remote state of player with `[player] sync_player_interval = "10s"` ([#18]).
- Much more helpful guidance when using `!song` incorrectly.
- Store aliases in the database instead of the configuration. See the [`alias configuration`] for more details ([#24]).
- Start storing some settings in the database ([#19]).
  * Bot keeps track of first time it's being started to perform first-time configuration.
- Promotions through the `promotions` module. See the [`promotions configuration`] for more details ([#25]).

### Changed
- Cleaned up old cruft in the codebase (`gfx` module).
- Moved log configuration to external file (see [example log4rs.yaml]).
- No longer raise an error on bad input.
- UI is now built in React ([#23]).
  * This adds the `-WebRoot` option to [`tools/setmod.ps1`] to override where to load files from for development purposes.
- `.oauth2` state is now stored in the database under settings.

### Removed
- Removed `!counter` in favor of `!command` with same functionality. Using the `{{count}}` variable in the template will cause the count to be incremented.
- `[[aliases]]` section from configuration. Aliases are now stored in the database. The first time you run the bot it will migrate all the aliases into the database.

[`alias configuration`]: README.md#alias
[`promotions configuration`]: README.md#promotions
[example log4rs.yaml]: log4rs.yaml
[`tools/setmod.ps1`]: tools/setmod.ps1

[#18]: https://github.com/udoprog/setmod/issues/18
[#19]: https://github.com/udoprog/setmod/issues/19
[#22]: https://github.com/udoprog/setmod/issues/22
[#23]: https://github.com/udoprog/setmod/issues/23
[#24]: https://github.com/udoprog/setmod/issues/24
[#25]: https://github.com/udoprog/setmod/issues/25

[0.2.4]: https://github.com/udoprog/setmod/compare/0.2.3...0.2.4

## [0.2.3]

### Added
- `!water` command that can be enabled using as a module through `[[modules]]` see [README.md](README.md#water).
- Attempt to automatically refresh expired tokens on startup ([#21]).

### Changed
- Move all locks to [`parking_lot`]
- Improved logic to notify on device configuration.

[`parking_lot`]: https://github.com/Amanieu/parking_lot

[#21]: https://github.com/udoprog/setmod/issues/21

[0.2.3]: https://github.com/udoprog/setmod/compare/0.2.2...0.2.3

## [0.2.2]

### Added
- Player now plays music through Spotify's blessed Connect API ([#17]).
- `!swearjar` command that can be enabled using as a module through `[[modules]]` see [README.md](README.md#swearjar).
- `!countdown` command that can be enabled using as a module through `[[modules]]` see [README.md](README.md#countdown).

### Changed
- Remove dependency on bundled `.dll` files.
- Deprecated the use of the `native` player in favor of `connect` since it's a potential TOS violation ([#17]).
- Improved administration UI:
  - Support for selecting Audio Device (does not persist across reboots) ([#20]).
  - Informing you more clearly when you need to authenticate.
  - Provide hint on how to configure persistent device.

[#17]: https://github.com/udoprog/setmod/issues/17
[#20]: https://github.com/udoprog/setmod/issues/20

[0.2.2]: https://github.com/udoprog/setmod/compare/0.2.1...0.2.2

## [0.2.1]

### Added
- `!song promote <number>` to promote songs to the top of the queue ([#2]).
- Optional web page (hosted on `setbac.tv`) to display current queue ([#3]) (config: `api_url`).
- Support suppressing echoing of current song (config: `[player] echo_current_song = false`).
- Show queue position in player view ([#8]).
- Support deleting a song at the given position ([#7]).
- Moderator action cooldowns ([#6]).
- Moderators are automatically picked up through `/mods` command on IRC ([#5]).
- Support for `!clip` command ([#13])
- Support for `!8ball` command ([#14])
- `!afterstream` command now has a cooldown configured through `[irc] afterstream_cooldown`.
- `!song when` to see when your requested song will be playing.
- Added `{{elapsed}}` as a variable for `current_song`.
- `[current_song] update_interval = "5s"` to specify how frequently the current song information will be updated.
  This might be necessary in case `{{elapsed}}` is used as a variable and you want it to update live.
- `[irc] startup_message = "HeyGuys"` to send a message when the bot starts.

### Changed
- Changed configuration format to be TOML and flatten it (see [example configuration]).
- Removed HTML escapes from `current_song` ([#4]).
- Fixed `!song purge` not sending update to `setbac.tv` ([#9]).
- Streamer is immune to cooldown and is always moderator ([#10]).
- Changed configuration format to flatten it more ([#11]).
- Reduced the number of scopes requested for tokens to a minimum.

[example configuration]: https://github.com/udoprog/setmod/blob/master/config.toml
[#2]: https://github.com/udoprog/setmod/issues/2
[#3]: https://github.com/udoprog/setmod/issues/3
[#4]: https://github.com/udoprog/setmod/issues/4
[#5]: https://github.com/udoprog/setmod/issues/5
[#6]: https://github.com/udoprog/setmod/issues/6
[#7]: https://github.com/udoprog/setmod/issues/7
[#8]: https://github.com/udoprog/setmod/issues/8
[#9]: https://github.com/udoprog/setmod/issues/9
[#10]: https://github.com/udoprog/setmod/issues/10
[#11]: https://github.com/udoprog/setmod/issues/11
[#13]: https://github.com/udoprog/setmod/issues/13
[#14]: https://github.com/udoprog/setmod/issues/14

[0.2.1]: https://github.com/udoprog/setmod/compare/0.0.1...0.2.1
