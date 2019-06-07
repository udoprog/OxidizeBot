# ![alt text](https://raw.githubusercontent.com/udoprog/setmod/master/bot/res/icon48.png "SetMod Rust Bot") SetMod

[![Build Status](https://travis-ci.org/udoprog/setmod.svg?branch=master)](https://travis-ci.org/udoprog/setmod)
[![Build status](https://ci.appveyor.com/api/projects/status/cxagsq3idti252a4/branch/master?svg=true)](https://ci.appveyor.com/project/udoprog/setmod/branch/master)

This is a high performance Twitch Bot written in Rust.

## Installing and Running

You can download an installer or an archive from [releases](https://github.com/udoprog/setmod/releases) or [build the project yourself](#building).

If you installed using an installer, SetMod will be in your start folder.
Once started, it shows up as a systray icon.

If you use an archive, you can unpack it in any directory.

It is suggested that you run the bot through `setmod.ps1` since that will run the bot in a loop in case it crashes or is shut down through Twitch.
On Windows, this can be done by right clicking and selecting `Run with PowerShell`.

## Migrating from 0.2 to 0.3

SetMod 0.3 completely removes the need for any configuration files.
Everything is now managed through the `Settings` page in the UI.

We've also moved where we expect the database to be, so if you have an old `0.2` database and a `config.toml` file you'll have to move it like this:

1. Install SetMod `0.3.x`
2. Start SetMod through the Start Menu.
3. When SetMod is running it has a systray icon.
   Click on it and select `Open Directory...`.
4. Quit SetMod.
5. Copy the following files into the directory that you just opened:
   * Your old `config.toml`
   * Your old `setmod.sql` database.
6. Start SetMod again. This time it will migrate any existing configuration.
7. Remove `config.toml`.

## Building

You'll need Rust and a working compiler: https://rustup.rs/

For now and until `async_await` is stable, you will need to use the _nightly_ rust compiler.
This can be installed and configured by running:

```
rustup toolchain install nightly
rustup default nightly
```

On **Windows**, you will need to setup some environment variables.
You can do that in PowerShell by running the following in the shell:

```
./tools/env.ps1
```

After this, you build the project using cargo:

```
cargo +nightly build --release
```

If you want to build and run the project in one go, there is a helper script in [`tools/setmod.ps1`] that you can run from anywhere in a powershell terminal, like this:

```
C:\setmod\> C:\Projects\setmod\tools\setmod.ps1
```

[`tools/setmod.ps1`]: tools/setmod.ps1

## Settings

SetMod is moving towards storing settings in the database.

These settings are stored as slash-separated strings, like `player/max-songs-per-user`.

You can find all available settings and their types in [`settings.yaml`](bot/src/settings.yaml).

When the bot is running, you can find all settings under `Internal -> Settings`.

## YouTube Player

setmod has support for playing YouTube videos.

This is enabled through the `song/youtube/support` setting and requires you to run the YouTube Player in the web UI.

This can be embedded in OBS with the following Custom CSS:

```css
body { background-color: rgba(0, 0, 0, 0); }
.overlay-hidden { display: none };
```

This will cause the player to disappear while it is not playing anything.

## Built-in Commands

#### `!admin`

All admin commands are restricted to **moderators**.

* `!admin version` - Responds with the current version of the setmod-bot package.
* `!admin refresh-mods` - Refresh the set of moderators in the bot. This is required if someone is modded or unmodded while the bot is running.
* `!admin settings <key>` - Read the value of a setting.
* `!admin settings <key> <value>` - Write the value of a setting.
* `!admin push` - Push a value to a setting which is a collection.
* `!admin delete <key> <value>` - Delete a value from a settings which is a collection.
* `!admin shutdown` - Cause the mod to cleanly shut down, and hopefully being restarted by the management process.
* `!admin enable-group <group>` - Enable all commands, aliases, and promotions part of the specified group.
* `!admin disable-group <group>` - Disable all commands, aliases, and promotions part of the specified group.

## Bad Words

Bad words filter looks at all words in a channel, converts them to singular and matches them phonetically against a word list.

The word list can be stored in the database with the `!badword edit <why>` command.
But you can also use a `bad_words.yml` file that looks like this:

```yaml
words:
 - word: cat
   why: Please don't talk about cats here {{name}} -___-
 - word: trump
```

If a word matches, the message will be deleted.

`why` is optional, but will be communicated to the user in case their message is deleted.
It supports the following template variables:

* `{{name}}` - the user who said the word.
* `{{target}}` - the channel where the word was sent.

## Commands

Every command is enabled through a Setting named `<command>/enabled`.

To for example enable the `!admin` command, you'd have to make sure the `admin/enabled` setting is set.

#### Misc Commands

You enable the `!admin` command by setting `admin/enabled` to `true`.

Available commands:

* `!uptime` - Get the current uptime.
* `!title` - Get the current title.
* `!title <title>` - Update the title to be `<title>`.
* `!game` - Get the current game.
* `!game <game>` - Update the game to be `<game>`.

#### `!command` command

You enable custom command administration by setting `command/enabled` to `true`.

Allows setting and requesting custom commands.

A command is the bot responding with a pre-defined message based on a template.

Available commands:

* `!command edit <name> <what>` - Set the command `<name>` to respond with `<what>` (**moderator**).
* `!command clear-group <name>` - Clear the group for command `<name>` (**moderator**).
* `!command group <name>` - Get the group the given command belongs to (**moderator**).
* `!command group <name> <group>` - Set the command `<name>` to be in the group `<group>` (**moderator**).
* `!command delete <name>` - Delete the command named `<name>` (**moderator**).
* `!command rename <from> <to>` - Rename the command `<from>` to `<to>` (**moderator**).

Template variables that can be used in `<what>`:

* `{{count}}` - The number of times the command has been invoked.
* `{{name}}` - The user who said the word.
* `{{target}}` - The channel where the word was sent.

#### `!alias` command

Allows setting custom aliases.
Aliases are prefixes that when invoked they will be expanded when processed by the bot.

For example, lets say we have an alias named `!sr` configured to `!song request {{rest}}`.
This would allow us to invoke `!sr don't call me` and it would be processed as `!song request don't call me`.

Available commands:

* `!alias edit <name> <what>` - Set the command `<name>` to alias to `<what>` (**moderator**).
* `!alias clear-group <name>` - Clear the group for alias `<name>` (**moderator**).
* `!alias group <name>` - Get the group the given alias belongs to (**moderator**).
* `!alias group <name> <group>` - Set the alias `<name>` to be in the group `<group>` (**moderator**).
* `!alias delete <name>` - Delete the command named `<name>` (**moderator**).
* `!alias rename <from> <to>` - Rename the command `<from>` to `<to>` (**moderator**).

Template variables that can be used in `<what>`:

* `{{rest}}` - The rest of the line being passed in.
* `{{name}}` - The user who invoked the alias.
* `{{target}}` - The channel where the alias was invoked.

###### Deprecated configuration `[[aliases]]`

Aliases used to be specified in the configuration.
If these are still present, the bot will migrate those aliases into the database and post a warning at startup.

The configuration used to look like this:

```toml
[[aliases]]
match = "!sr"
replace = "!song request {{rest}}"

[[aliases]]
match = "!sl"
replace = "!song list {{rest}}"

[[aliases]]
match = "!volume"
replace = "!song volume {{rest}}"
```

Now it's all handled using the `!alias` command.

#### `!afterstream` command

You enable the `!afterstream` command by setting `afterstream/enabled` to `true`.

Enabled adding afterstream messages.

Afterstream messages keeps track of who added them and when.

Available commands:

* `!afterstream <message>` - Leaves the `<message>` in the afterstream queue.

Afterstreams that are posted are made available in the UI at: http://localhost:12345/after-streams


#### `!song` command

You enable the `!song` command by setting `song/enabled` to `true`.

Enables song playback through Spotify.

Available commands:

* `!song request spotify:track:<id>` - Request a song through a Spotify URI.
* `!song request https://open.spotify.com/track/<id>` - Request a song by spotify URL.
* `!song request <search>` - Request a song by searching for it. The first hit will be used.
* `!song skip` - Skip the current song (**moderator**).
* `!song play` - Play the current song (**moderator**).
* `!song pause` - Pause the current song (**moderator**).
* `!song toggle` - Toggle the current song (Pause/Play) (**moderator**).
* `!song volume` - Get the current volume.
* `!song volume <volume>` - Set the current volume to `<volume>` (**moderator**).
* `!song length` - Get the current length of the queue.
* `!song current` - Get information on the current song.
* `!song delete last` - Delete the last song in the queue (**moderator**).
* `!song delete last <user>` - Delete the last song in the queue added by the given `<user>` (**moderator**).
* `!song delete mine` - A user is allowed to delete the last song that _they_ added.
* `!song delete <position>` - Delete a song at the given position (**moderator**).
* `!song list` - Get the next three songs.
* `!song list <n>` - Get the next `<n>` songs (**moderator**).
* `!song theme <name>` - Play the specified theme song (**moderator**).
* `!song close [reason]` - Close the song queue with an optional `[reason]` (**moderator**).
* `!song open` - Open the song queue (**moderator**).
* `!song promote <number>` - Promote the song at the given position `<number>` in the queue (**moderator**).
* `!song when` - Find out when your song will play.
* `!song when <user>` - Find out when the song for a specific user will play (**moderator**).

#### `!clip` command

You enable the `!clip` command by setting `clip/enabled` to `true`.

The `!clip` command enables the `!clip` command.

This command has a cooldown determined by the `[irc] clip_cooldown` configuration key (see above).

#### `!8ball` command

You enable the `!8ball` command by setting `8ball/enabled` to `true`.

Enables the Magic `!8ball` command. Cause it's MAGIC.

#### `currency`

Enables a loyalty currency system and a couple of commands.

A currency is enabled by adding the following to your configuration:

```toml
[currency]
name = "thingies"
```

Enabled commands depend on the `name` of your currency, so we are gonna assume the currency is currently named `thingies`:

- `!thingies` - Get your current balance.
- `!thingies give <user> <amount>` - Give `<user>` `<amount>` of the given currency. This will _transfer_ the specified amount from your account to another.
- `!thingies boost <user> <amount>` - Give the specified `<user>` an `<amount>` of currency. Can be negative to take away (**moderator**).
- `!thingies windfall <amount>` - Give away `<amount>` currency to all current viewers (**moderator**).
- `!thingies show <user>` - Show the amount of currency for the given user (**moderator**).

#### `!swearjar` command

You enable the `!swearjar` command by setting `swearjar/enabled` to `true`.

This also requires the `!currency` command to be enabled.

Available commands:

* `!swearjar` - Anyone can invoke the swearjar to reward all viewers with some currency from the streamer when they swear.

#### `!countdown` command

You enable the `!countdown` command by setting `countdown/enabled` to `true`.

The `!countdown` command allows setting a countdown and a corresponding template, that will be written to a file while the countdown is active.

The following settings are required:

* `countdown/path` - The path to write the countdown to.

Available commands:

* `!countdown set <duration> <template>` - Set a countdown, available template variables are `{{remaining}}`, `{{duration}}`, and `{{elapsed}}`.
  - Example: `!countdown set 5m I'll be live in {{remaining}}`
  - Example: `!countdown set 1m Getting food, back in {{remaining}}`
* `!countdown clear` - Clear the current countdown.

#### `!water` command

You enable the `!water` command by setting `water/enabled` to `true`.

Available commands:

* `!water` - A user can remind the streamer to drink water and will be rewarded one unit of stream currency for every minute since last reminder.
* `!water undo` - Undos the last water reminder and refunds the reward.

#### `!promo` command

You enable the `!promo` command by setting `promo/enabled` to `true`.

The following settings are required:

* `promo/frequency` - The highest frequency at which promotions are posted.

Available commands:

* `!promo list` - List all available promotions.
* `!promo edit <id> <frequency> <what>` - Set the promotion identified by `<id>` to send the message `<what>` every `<frequency>`.
  - Example: `!promo edit discord 30m Hey, did you know I have a Discord? Join it at http://example.com!`
* `!promo clear-group <name>` - Clear the group for promotion `<name>` (**moderator**).
* `!promo group <name>` - Get the group the given promotion belongs to (**moderator**).
* `!promo group <name> <group>` - Set the promotion `<name>` to be in the group `<group>` (**moderator**).
* `!promo delete <id>` - Delete the promotion with the given id.
* `!promo rename <from> <to>` - Delete the promotion with the given id.

#### `!gtav` command

You enable the `!gtav` command by setting `gtav/enabled` to `true`.

The `gtav` module enables support for [`ChaosMod`](https://github.com/udoprog/ChaosMod).

This has a lot of settings to tweak, go into `Settings` and search for `gtav` to find out more.
It also enables a lot of commands.
Go to https://bit.ly/gtavchaos for a full list.

All of these have different effects and costs (which requires the `!currency` command).

#### `!speedrun` command

You enable the `!speedrun` command by setting `speedrun/enabled` to `true`.

* `!speedrun record <game> [filters]` - List a specific record.
  * Example: `!speedrun record gtav --user burhac --category 100%`

Available `[filters]` are:
* `--user <name>` - Limit results to the given user.
* `--category <name>` - Limit results to the given category.
* `--sub-category <name>` - Limit results to the given sub-category.
* `--misc` - Include misc categories.
* `--misc-only` - Only list misc categories.
