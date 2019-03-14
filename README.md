# setmod

This is a high performance Twitch Bot written in Rust.

## Building on 64-bit Windows

You'll need Rust and a working compiler: https://rustup.rs/

Building requires setting the necessary environment variables to point out libraries.
You can set them in PowerShell by running:

```
. ./Env.ps1
```

After this, build using cargo:

```
cargo build --release
```

## Configuration

Two configuration files are neccessary.

First, `config.yml`:

```yaml
streamer: setbac

irc:
  bot: setmod
  channels:
    - name: "#setbac"
      # Features enabled for the channel.
      # Removing features will remove their corresponding commands.
      features:
        - song
        - admin
        - command
        - counter
        - afterstream
        - bad-words
        - url-whitelist
      # Loyalty currency used.
      # Remove to disable.
      currency:
        name: ether
      # Player configuration.
      # To find out available speakers, change `speaker` to `"?"` and available speakers will be listed when running the
      # command.
      player:
        speaker: "Speakers (Realtek High Definiti"
        volume: 50
      # Aliases, only single-word command aliases are currently available.
      aliases:
      - match: "!sr"
        replace: "!song request {{rest}}"
      - match: "!sl"
        replace: "!song list {{rest}}"
      - match: "!volume"
        replace: "!song volume {{rest}}"
  # List of moderators allowed to perform moderator actions.
  moderators:
    - "setbac"
    - "setmod"
  # Whitelisted hosts (if url-whitelist is enabled).
  whitelisted_hosts:
    - "youtube.com"
    - "youtu.be"

# Optional bad words list.
# See "Bad Words" below.
bad_words: bad_words.yml

# Path to database (technicaly an connection URL, but only sqlite supported right now).
database_url: database.sql
```

Second, `secrets.yml`:

```yaml
spotify::native:
  username: USERNAME
  password: PASSWORD

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

Features are enabled per-channel like this:

```yaml
irc:
  channels:
    - name: "#mychannel"
      features:
        - name-of-feature
```

Where `name-of-feature` is one of the features listed below.

#### `admin`

Enabled commands:

* `!uptime` - Get the current uptime.
* `!title` - Get the current title.
* `!title <title>` - Update the title to be `<title>`.
* `!game` - Get the current game.
* `!game <game>` - Update the game to be `<game>`.

#### `command`

Allows setting and requesting custom commands.

The command named `foo` would be invoked `!foo`.

A command is the bot responding with a pre-defined message based on a template.

Enabled commands:

* `!command edit <name> <what>` - Set the command `!<name>` to respond with `<what>` (**moderator**)..
* `!command delete <name>` - Delete the command named `<name>` (**moderator**)..

Template variables that can be used in `<what>`:

* `{{name}}` - the user who said the word.
* `{{target}}` - the channel where the word was sent.

#### `counter`

Identical to `command` but keeps track of a counter of how many times it has been invoked.

The counter named `foo` would be invoked `!foo`.

A command is the bot responding with a pre-defined message based on a template.

Enabled commands:

* `!counter edit <name> <what>` - Set the command `!<name>` to respond with `<what>` (**moderator**)..
* `!counter delete <name>` - Delete the command named `<name>` (**moderator**)..

Template variables that can be used in `<what>`:

* `{{count}}` - The number of times the counter has been invoked.
* `{{name}}` - The user who said the word.
* `{{target}}` - The channel where the word was sent.

#### `afterstream`

Add an afterstream message.

Afterstream messages keeps track of who added them and when.

Enabled commands:

* `!afterstream <message>` - Leaves the `<message>` in the afterstream queue.

Note: the only way to list afterstream messages right now is to read the database:

```
sqlite3.exe database.sql
sqlite> select * from after_streams;

...
```

#### `song`

Enables song playback through Spotify.

Enabled commands:

* `!song request spotif:track:<id>` - Request a song through a Spotify URI.
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
* `!song list` - Get the next three songs.
* `!song list <n>` - Get the next `<n>` songs (**moderator**).

## Aliases

Aliases are enabled per-channel like this:

```yaml
irc:
  channels:
    - name: "#mychannel"
      aliases:
      - match: "!sr"
        replace: "!song request {{rest}}"
      - match: "!sl"
        replace: "!song list {{rest}}"
      - match: "!volume"
        replace: "!song volume {{rest}}"
```

Aliases are applied as a pre-processing step for each incoming command.