# setmod

[![Build Status](https://travis-ci.org/udoprog/setmod.svg?branch=master)](https://travis-ci.org/udoprog/setmod)
[![Build status](https://ci.appveyor.com/api/projects/status/cxagsq3idti252a4/branch/master?svg=true)](https://ci.appveyor.com/project/udoprog/setmod/branch/master)

This is a high performance Twitch Bot written in Rust.

## Installing and Running

You can download an archive from [releases](https://github.com/udoprog/setmod/releases) or [build the project yourself](#building).

If you use an archive, you can unpack it in any directory.

It is suggested that you run the bot through `setmod.ps1` since that will run the bot in a loop in case it crashes or is shut down through Twitch.
On Windows, this can be done by right clicking and selecting `Run with PowerShell`.

Before the bot can run, you need to set it up.
See the next section.

## Setting Up

Two configuration files are neccessary.

First, `config.toml`. You can use the [config.toml.example] as a base which should be included in your distribution.

Second, `secrets.yml` which specifies secrets for your bot. **This file is very sensitive**, you must avoid sharing it with anyone.

```yaml
spotify::oauth2:
  client_id: SPOTIFY_CLIENT_ID
  client_secret: SPOTIFY_CLIENT_SECRET

twitch::oauth2:
  client_id: TWITCH_CLIENT_ID
  client_secret: TWITCH_CLIENT_SECRET
```

The client ids and secrets above must be registered on Twitch and Spotify respectively:

* Twitch: https://dev.twitch.tv/console/apps/create
* Spotify: https://developer.spotify.com/dashboard/

For both of these, you must add the following redirect URL:

```
http://localhost:12345/redirect
```

This is where the bot will be running while it is receiving tokens.

[config.toml.example]: config.toml.example

## Building

You'll need Rust and a working compiler: https://rustup.rs/

After this, you build the project using cargo:

```
cargo build --release
```

If you want to build and run the project in one go, there is a helper script in [`tools/setmod.ps1`] that you can run from anywhere in a powershell terminal, like this:

```
C:\setmod\> C:\Projects\setmod\tools\setmod.ps1
```

[`tools/setmod.ps1`]: tools/setmod.ps1

## Settings

SetMod is moving towards storing settings in the database.

These settings are stored as slash-separated strings, like `player/max-songs-per-user`.

The available settings are:

#### `first-run`

Indicates if the bit has run before or not.
This is used to determine whether the bot should do first time behavior on startup (like opening the browser).

#### `irc/idle-detection/threshold`

Determines how many messages must bee seen to not consider the channel idle.

Idle detection is used by the `!promo` feature.

#### `migration/aliases-migrated`

Indicates if `[[aliases]]` has been migrated from the configuration file.

This setting can be removed if you remove the `[[aliases]]` section from the configuration file.

#### `player/max-queue-length`

The max queue length permitted by the player.

#### `player/max-songs-per-user`

The max number of songs permitted by non-mod users.

#### `promotions/frequency`

The frequency at which to run promotions.

#### `secrets/*`

Secret things which are stored in the database, like tokens.

## Built-in Commands

#### `!admin`

All admin commands are restricted to **moderators**.

* `!admin version` - Responds with the current version of the setmod-bot package.
* `!admin refresh-mods` - Refresh the set of moderators in the bot. This is required if someone is modded or unmodded while the bot is running.
* `!admin shutdown` - Cause the mod to cleanly shut down, and hopefully being restarted by the management process.

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

## Features

Note: In a future version, `features` will be removed in favor of `[[modules]]`. See below.

Features are enabled per-channel like this:

```toml
features = [
    "name-of-feature"
]
```

Where `name-of-feature` is one of the features listed below.

#### `admin` feature

You enable the feature by adding `"admin"` to the `features` array in the configuration:

```toml
features = [
  "admin",
]
```

Enabled commands:

* `!uptime` - Get the current uptime.
* `!title` - Get the current title.
* `!title <title>` - Update the title to be `<title>`.
* `!game` - Get the current game.
* `!game <game>` - Update the game to be `<game>`.

#### `command` feature

You enable the feature by adding `"command"` to the `features` array in the configuration:

```toml
features = [
  "command",
]
```

Allows setting and requesting custom commands.

A command is the bot responding with a pre-defined message based on a template.

Enabled commands:

* `!command edit <name> <what>` - Set the command `<name>` to respond with `<what>` (**moderator**).
* `!command delete <name>` - Delete the command named `<name>` (**moderator**).
* `!command rename <from> <to>` - Rename the command `<from>` to `<to>` (**moderator**).

Template variables that can be used in `<what>`:

* `{{count}}` - The number of times the command has been invoked.
* `{{name}}` - The user who said the word.
* `{{target}}` - The channel where the word was sent.

#### `alias` feature

This feature is enabled by default.

Allows setting custom aliases.
Aliases are prefixes that when invoked they will be expanded when processed by the bot.

For example, lets say we have an alias named `!sr` configured to `!song request {{rest}}`.
This would allow us to invoke `!sr don't call me` and it would be processed as `!song request don't call me`.

Enabled commands:

* `!alias edit <name> <what>` - Set the command `<name>` to alias to `<what>` (**moderator**).
* `!alias delete <name>` - Delete the command named `<name>` (**moderator**).
* `!alias rename <from> <to>` - Rename the command `<from>` to `<to>` (**moderator**).

Template variables that can be used in `<what>`:

* `{{rest}}` - The rest of the line being passed in.
* `{{name}}` - The user who invoked the alias.
* `{{target}}` - The channel where the alias was invoked.

###### Deprecated configuration

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

#### `afterstream` feature

You enable the feature by adding `"afterstream"` to the `features` array in the configuration:

```toml
features = [
  "afterstream",
]
```

Enabled adding afterstream messages.

Afterstream messages keeps track of who added them and when.

Enabled commands:

* `!afterstream <message>` - Leaves the `<message>` in the afterstream queue.

Afterstreams that are posted are made available in the UI at: http://localhost:12345/after-streams


#### `song` feature

You enable the feature by adding `"song"` to the `features` array in the configuration:

```toml
features = [
  "song",
]
```

Enables song playback through Spotify.

Enabled commands:

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

#### `clip` feature

You enable the feature by adding `"clip"` to the `features` array in the configuration:

```toml
features = [
  "clip",
]
```

The `clip` feature enables the `!clip` command.

This command has a cooldown determined by the `[irc] clip_cooldown` configuration key (see above).

#### `8ball` feature

You enable the feature by adding `"8ball"` to the `features` array in the configuration:

```toml
features = [
  "8ball",
]
```

Enables the Magic `!8ball` command. Cause it's MAGIC.

## Modules

Modules are defined in the `[[modules]]` sections of the configuration.

They enable certain behavior of the bot, and are generally better than `features` since they allow adding configuration
associated with the module.

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

#### `swearjar`

You enable the `swearjar` module by adding the following to your configuration:

```toml
[[modules]]
type = "swearjar"

# The amount of currency to reward all watchers with.
reward = 10
# Cooldown between invocations, default: 1m
# cooldown = "1m"
```

This also requires the `currency` feature to be enabled.

Enabled commands:

* `!swearjar` - Anyone can invoke the swearjar to reward all viewers with some currency from the streamer when they swear.

#### `countdown`

You enable the `countdown` module by adding the following to your configuration:

```toml
[[modules]]
type = "countdown"
path = "E:\\temp\\countdown.txt"
```

Enabled commands:

* `!countdown set <duration> <template>` - Set a countdown, available template variables are `{{remaining}}`, `{{duration}}`, and `{{elapsed}}`.
  - Example: `!countdown set 5m I'll be live in {{remaining}}`
  - Example: `!countdown set 1m Getting food, back in {{remaining}}`
* `!countdown clear` - Clear the current countdown.

#### `water`

You enable the `water` module by adding the following to your configuration:

```toml
[[modules]]
type = "water"
```

Enabled commands:

* `!water` - A user can remind the streamer to drink water and will be rewarded one unit of stream currency for every minute since last reminder.
* `!water undo` - Undos the last water reminder and refunds the reward.

#### `promotions`

You enable the `promotions` module by adding the following to your configuration:

```toml
[[modules]]
type = "promotions"
frequency = "15m"
```

The frequency says how frequently promotions should be posted.
This is also combined with a custom frequency that must be met per promotion.

Enabled commands:

* `!promo list` - List all available promotions.
* `!promo edit <id> <frequency> <what>` - Set the promotion identified by `<id>` to send the message `<what>` every `<frequency>`.
  - Example: `!promo edit discord 30m Hey, did you know I have a Discord? Join it at http://example.com!`
* `!promo delete <id>` - Delete the promotion with the given id.
* `!promo rename <from> <to>` - Delete the promotion with the given id.

#### `gtav`

The `gtav` module enables support for [`ChaosMod`](https://github.com/udoprog/ChaosMod).

It enables _a lot_ of commands:

* `!gtav other randomize-color`
* `!gtav other randomize-weather`
* `!gtav other randomize-character`
* `!gtav other license <name>`
* `!gtav reward car`
* `!gtav reward vehicle`
* `!gtav reward repair`
* `!gtav reward wanted`
* `!gtav reward weapon`
* `!gtav reward health`
* `!gtav reward armor`
* `!gtav reward parachute`
* `!gtav reward boost`
* `!gtav reward superboost`
* `!gtav reward superspeed`
* `!gtav reward superswim`
* `!gtav reward superjump`
* `!gtav reward invincibility`
* `!gtav reward ammo`
* `!gtav reward exploding-bullets`
* `!gtav reward matrix-slam`
* `!gtav reward mod-vehicle`
* `!gtav punish wanted 1`
* `!gtav punish wanted 2`
* `!gtav punish wanted 3`
* `!gtav punish wanted 4`
* `!gtav punish wanted 5`
* `!gtav punish stumble`
* `!gtav punish fall`
* `!gtav punish health`
* `!gtav punish engine`
* `!gtav punish tires`
* `!gtav punish weapon`
* `!gtav punish all-weapons`
* `!gtav punish brake`
* `!gtav punish ammo`
* `!gtav punish enemy`
* `!gtav punish enemy <amount>`
* `!gtav punish drunk`
* `!gtav punish very-drunk`
* `!gtav punish set-on-fire`
* `!gtav punish set-peds-on-fire`
* `!gtav punish make-peds-aggressive`
* `!gtav punish disable-control`

All of these have different effects and costs (which requires the `currency` feature).

These are mostly detailed here: https://bit.ly/gtavchaos
