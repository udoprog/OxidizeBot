# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/udoprog/setmod/compare/0.0.1...HEAD
