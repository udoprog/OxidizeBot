/******/ (() => { // webpackBootstrap
/******/ 	var __webpack_modules__ = ({

/***/ 1484:
/***/ ((module) => {

module.exports = {
	"groups": [
		{
			"name": "Documentation",
			"content": "Commands for receiving command help in OxidizeBot.\n",
			"commands": [
				{
					"name": "!help",
					"content": "Links to this help page."
				},
				{
					"name": "!help `<topic...>`",
					"content": "Get help for a specific `<topic...>`, like `!song`.",
					"examples": [
						{
							"name": "Help for the song command",
							"content": "setbac: !help !song\nSetMod: setbac -> For help on that, go to https://setbac.tv/help?q=%21song\n"
						}
					]
				}
			]
		},
		{
			"name": "Admin Commands",
			"content": "Commands for changing how the bot behaves.",
			"commands": [
				{
					"name": "!admin version",
					"content": "Responds with the current version of Oxidize Bot package.",
					"examples": [
						{
							"name": "The output of the command.",
							"content": "setbac: !admin version\nSetMod: setbac -> OxidizeBot Version 1.0.0-beta.21\n"
						}
					]
				},
				{
					"name": "!admin refresh-mods",
					"content": "Refresh the set of moderators in the bot. This is required if someone is modded or unmodded while the bot is running."
				},
				{
					"name": "!admin settings `<key>`",
					"content": "Read the value of a setting.",
					"examples": [
						{
							"name": "Getting the value of a configuration.",
							"content": "setbac: !admin settings afterstream/enabled\nSetMod: setbac -> afterstream/enabled = true\n"
						}
					]
				},
				{
					"name": "!admin settings `<key>` `<value>`",
					"content": "Write the value of a setting.",
					"examples": [
						{
							"name": "Disabling the afterstream command",
							"content": "setbac: !admin settings afterstream/enabled false\nSetMod: setbac -> afterstream/enabled = false\n"
						}
					]
				},
				{
					"name": "!admin push `<key>` `<value>`",
					"content": "Add the value to a setting which is a collection."
				},
				{
					"name": "!admin delete `<key>` `<value>`",
					"content": "Delete a value from a setting which is a collection."
				},
				{
					"name": "!admin toggle `<key>`",
					"content": "Toggles the value of the specified setting if it's a boolean.",
					"examples": [
						{
							"name": "Toggling if the song player is detached",
							"content": "setbac: !admin toggle player/detached\nSetMod: setbac -> Updated setting player/detached = true\nsetbac: !admin toggle player/detached\nSetMod: setbac -> Updated setting player/detached = false\n"
						}
					]
				},
				{
					"name": "!admin shutdown",
					"content": "Shutdown the bot, causing it to (hopefully) restart."
				},
				{
					"name": "!admin enable-group `<group>`",
					"content": "Enable all commands, aliases, and promotions part of the specified group."
				},
				{
					"name": "!admin disable-group `<group>`",
					"content": "Disable all commands, aliases, and promotions part of the specified group."
				}
			]
		},
		{
			"name": "Misc Commands",
			"content": "Various commands.",
			"commands": [
				{
					"name": "!uptime",
					"content": "Get the amount of time that the stream has been live for.",
					"examples": [
						{
							"name": "The output of the uptime command.",
							"content": "setbac: !uptime\nSetMod: setbac -> Stream has been live for 5h 1m 21s.\n"
						}
					]
				},
				{
					"name": "!title",
					"content": "Get the current title of the stream."
				},
				{
					"name": "!title `<title>`",
					"content": "Set the title of the stream."
				},
				{
					"name": "!game",
					"content": " Get the current game of the stream."
				},
				{
					"name": "!game `<game>`",
					"content": "Set the game of the stream."
				}
			]
		},
		{
			"name": "!command",
			"content": "Commands related to custom command administration.\n",
			"commands": [
				{
					"name": "!command edit `<name>` `<template...>`",
					"content": "Set the command `<name>` to respond with `<template...>`.\n\n`<template...>` can use the following variables:\n\n* `{{count}}` - The number of times the command has been invoked.\n* `{{name}}` - The user who invoked the command.\n* `{{target}}` - The channel where the word was sent.\n* regex capture groups - Like `{{0}}` or `{{1}}` if a pattern used (see `!command pattern`).\n",
					"examples": [
						{
							"name": "Setting and using a command",
							"content": "setbac: !command edit !github {{name}} -> Visit my github at https://github.com/udoprog\nSetMod: setbac -> Edited command.\nsetbac: !github\nSetMod: setbac -> Visit my github at https://github.com/udoprog\n"
						}
					]
				},
				{
					"name": "!command pattern `<name>` `<pattern...>`",
					"content": "Set the command `<name>` to respond when it matches the regular expression in `<pattern...>`.\n\nPatterns can define capture groups, which will be made available to `<template...>` through `{{0}}`, `{{1}}`, etc...\n",
					"examples": [
						{
							"name": "Setting and using a command with a pattern",
							"content": "setbac: !command edit why {{name}} -> Because it's faster...\nSetMod: setbac -> Edited command.\nsetbac: !command pattern why (?i)why.*\\?\nSetMod: setbac -> Edited pattern for command.\nsetbac: Why are you doing this?\nSetMod: setbac -> Because it's faster...\n"
						},
						{
							"name": "Using a capture group in the template",
							"content": "setbac: !command edit why {{name}} -> Because \"{{1}}\" is faster...\nSetMod: setbac -> Edited command.\nsetbac: !command pattern why (?i)why are you (.+)\\?\nSetMod: setbac -> Edited pattern for command.\nsetbac: Why are you taking that car?\nSetMod: setbac -> Because \"taking that car\" is faster...\n"
						}
					]
				},
				{
					"name": "!command pattern `<name>`",
					"content": "Clear the pattern from the given command `<name>`.\n"
				},
				{
					"name": "!command group `<name>`",
					"content": "Get the group the command `<name>` belongs to.\n"
				},
				{
					"name": "!command group `<name>` `<group>`",
					"content": "Add the command `<name>`  to the group `<group>`.\n"
				},
				{
					"name": "!command clear-group `<name>`",
					"content": "Remove the command `<name>` from all groups.\n"
				},
				{
					"name": "!command delete `<name>`",
					"content": "Delete the command `<name>`.\n"
				},
				{
					"name": "!command rename `<from>` `<to>`",
					"content": "Rename to command `<from>` to `<to>`.\n"
				}
			]
		},
		{
			"name": "!alias",
			"content": "Simple aliases which can be expanded to complex commands.\n\nThis is typically used to take a longer command like `!song request` and shorten it to something like `!sr`.\n",
			"commands": [
				{
					"name": "!alias edit `<name>` `<template...>`",
					"content": "Set the command `<name>` to alias to `<template...>`.\n\nIn the template you can use the variable `{{rest}}` to expand to the rest of the command being called.\n",
					"examples": [
						{
							"name": "Using a capture group in the template",
							"content": "setbac: !alias edit !sr !song request {{rest}}\nSetMod: setbac -> Edited alias.\nsetbac: !sr we will rock you\nSetMod: setbac -> Added \"We Will Rock You - Remastered\" by Queen at position #1!\n"
						}
					]
				},
				{
					"name": "!alias clear-group `<name>`",
					"content": "Remove the alias `<name>` from all groups.\n"
				},
				{
					"name": "!alias group `<name>`",
					"content": "Get the group the alias `<name>` belongs to, if any.\n"
				},
				{
					"name": "!alias group `<name>` `<group>`",
					"content": "Set the alias `<name>` to be in the group `<group>`.\n"
				},
				{
					"name": "!alias delete `<name>`",
					"content": "Delete the alias named `<name>`.\n"
				},
				{
					"name": "!alias rename `<from>` `<to>`",
					"content": "Rename the command `<from>` to `<to>`.\n"
				}
			]
		},
		{
			"name": "!afterstream",
			"content": "Adds an \"afterstream message\" to the bot, which will be available to the streamer after the stream is over.\n",
			"commands": [
				{
					"name": "!afterstream `<message...>`",
					"content": "Adds the message `<message...>` to be read by the streamer after the stream is over.\n\nMessages are avilable [in the After Streams page](http://localhost:12345/after-streams) of the bot.\n"
				}
			]
		},
		{
			"name": "Clip Command",
			"content": "Clips are small video snippets that can be created quickly on Twitch.\n",
			"commands": [
				{
					"name": "!clip",
					"content": "Creates a Twitch Clip 30 seconds long from the current time.\n"
				}
			]
		},
		{
			"name": "Song Commands",
			"content": "Commands to request and manage songs playing on stream.\n",
			"commands": [
				{
					"name": "!song request `spotify:track:<id>`",
					"content": "Request a song through a Spotify URI.\n"
				},
				{
					"name": "!song request `https://open.spotify.com/track/<id>`",
					"content": "Request a song by spotify URL.\n",
					"examples": [
						{
							"name": "Request a song from Spotify by URL",
							"content": "setbac: !song request https://open.spotify.com/track/4pbJqGIASGPr0ZpGpnWkDn\nSetMod: setbac -> Added \"We Will Rock You - Remastered\" by Queen at position #1!\n"
						}
					]
				},
				{
					"name": "!song request `<search>`",
					"content": "Request a song by searching for it. The first hit will be used.\n"
				},
				{
					"name": "!song skip",
					"content": "Skip the current song.\n"
				},
				{
					"name": "!song play",
					"content": "Play the current song.\n"
				},
				{
					"name": "!song pause",
					"content": "Pause the current song.\n"
				},
				{
					"name": "!song toggle",
					"content": "Toggle the current song (Pause/Play).\n"
				},
				{
					"name": "!song volume",
					"content": "Get the current volume.\n"
				},
				{
					"name": "!song volume `<volume>`",
					"content": "Set the current volume to `<volume>`.\n"
				},
				{
					"name": "!song length",
					"content": "Get the current length of the queue.\n"
				},
				{
					"name": "!song current",
					"content": "Get information on the current song in the queue.\n",
					"examples": [
						{
							"name": "Get the current song in the queue",
							"content": "setbac: !song current\nSetMod: setbac -> Current song: \"We Will Rock You - Remastered\" by Queen, requested by setbac - 01:19 / 02:02 - https://open.spotify.com/track/4pbJqGIASGPr0ZpGpnWkDn\n"
						}
					]
				},
				{
					"name": "!song purge",
					"content": "Deletes all songs in the queue\n",
					"examples": [
						{
							"name": "The output of the command.",
							"content": "setbac: !song purge\nSetMod: setbac -> Song queue purged.\n"
						}
					]
				},
				{
					"name": "!song delete last",
					"content": "Delete the last song in the queue.\n",
					"examples": [
						{
							"name": "Delete the last song in the queue",
							"content": "setbac: !song delete last\nSetMod: setbac -> Removed: \"We Will Rock You - Remastered\" by Queen!\n"
						}
					]
				},
				{
					"name": "!song delete last `<user>`",
					"content": "Delete the last song in the queue added by the given `<user>`.\n\nThis is typically only permitted by moderators.\n"
				},
				{
					"name": "!song delete mine",
					"content": "Delete the last song that _you_ added.\n\nAny user is allowed to delete their own songs.\n"
				},
				{
					"name": "!song delete `<position>`",
					"content": "Delete a song at the given `<position>`.\n"
				},
				{
					"name": "!song list",
					"content": "List the songs that will play.\n\nThis will usually take you to the appropriate player on https://setbac.tv/players - unless the streamer has configured it differently.\n"
				},
				{
					"name": "!song list `<n>`",
					"content": "List the next `<n>` songs in chat.\n\nThis will usually take you to the appropriate player on https://setbac.tv/players - unless the streamer has configured it differently.\n"
				},
				{
					"name": "!song theme `<name>`",
					"content": "Play the specified theme song by `<name>` (see `!theme` command).\n"
				},
				{
					"name": "!song close `[reason]`",
					"content": "Close the song queue with an optional `[reason]`.\n\nClosing the queue prevents subsequent song requests from being queued up.\n",
					"examples": [
						{
							"name": "Closing the queue",
							"content": "setbac: !song close We won't be rocking any more...\nSetMod: setbac -> Closed player from further requests.\nsetbactesting: !song request we will rock you\nSetMod: setbactesting -> We won't be rocking any more...\n"
						}
					]
				},
				{
					"name": "!song open",
					"content": "Open the song player for further requests.\n"
				},
				{
					"name": "!song promote `<position>`",
					"content": "Promote the song at the given `<position>` in the queue to the head of the queue, which is the next song that will play.\n"
				},
				{
					"name": "!song when",
					"content": "Find out when your song will play.\n"
				},
				{
					"name": "!song when `<user>`",
					"content": "Find out when the song for the given `<user>` will play.\n"
				}
			]
		},
		{
			"name": "8-Ball",
			"content": "A simple 8 ball which might or might not tell your fortune.\n",
			"commands": [
				{
					"name": "!8ball `<question...>`",
					"content": "Ask the 8 ball a `<question...>` and receive your fortune.\n",
					"examples": [
						{
							"name": "Asking the 8 ball a question",
							"content": "setbac: !8ball Will I eat lunch?\nSetMod: setbac -> Better not tell you now.\n"
						}
					]
				}
			]
		},
		{
			"name": "Currency Commands",
			"content": "Commands related to managing your _stream currency_.\n\nThe stream currency is named differently, and `thingies` below will match your currency.\n\nFor example, the stream currency of `setbac` is `ether`.\n",
			"commands": [
				{
					"name": "!currency",
					"content": "Get your current balance.\n",
					"examples": [
						{
							"name": "Getting your balance",
							"content": "setbac: !ether\nSetMod: setbac -> You have 40307 ether.\n"
						}
					]
				},
				{
					"name": "!currency give `<user>` `<amount>`",
					"content": "Give `<amount>` of stream currency to `<user>`.\n\nAnyone can do this, as long as they have the necessary balance.\n",
					"examples": [
						{
							"name": "setbac giving bdogs_gaming 100 ether",
							"content": "setbac: !ether give bdogs_gaming 100\nSetMod: setbac -> Gave bdogs_gaming 100 ether!\n"
						}
					]
				},
				{
					"name": "!currency boost `<user>` `<amount>`",
					"content": "Make up `<amount>` of currency, and give it to `<user>`.\n\nThis will create the currency out of nothing. Use sparingly!\n",
					"examples": [
						{
							"name": "`setbac` boosting `bdogs_gaming` with 100 ether",
							"content": "setbac: !ether boost bdogs_gaming 100\nSetMod: setbac -> Gave bdogs_gaming 100 ether!\n"
						}
					]
				},
				{
					"name": "!currency windfall `<amount>`",
					"content": "setbac: !ether windfall 10\n",
					"examples": [
						{
							"name": "`setbac` boosting everyone with 10 ether",
							"content": "setbac: !ether windfall 10\n* SetMod gave 10 ether to EVERYONE!\n"
						}
					]
				},
				{
					"name": "!currency show `<user>`",
					"content": "Show the balance for `<user>`.\n\nThis is typically only permitted by moderators.\n",
					"examples": [
						{
							"name": "`setbac` showing the balance of bdogs_gaming",
							"content": "setbac: !ether show bdogs_gaming\nSetMod: setbac -> bdogs_gaming has 390 ether.\n"
						}
					]
				}
			]
		},
		{
			"name": "Swearjar",
			"content": "If the streamer has a potty mouth that they wish to rid themselves of, they can make use of the swearjar command.\n",
			"commands": [
				{
					"name": "!swearjar",
					"content": "Invoke the swearjar. Causing the streamer to give all their viewers some stream currency.\n",
					"examples": [
						{
							"name": "`setbac` invoking the swearjar",
							"content": "setbac: !swearjar\n* SetMod has taken 110 ether from setbac and given it to the viewers for listening to their bad mouth!\n"
						}
					]
				}
			]
		},
		{
			"name": "Countdown",
			"content": "A simple command to keep track of a timer in a file.\n",
			"commands": [
				{
					"name": "!countdown set `<duration>` `<template...>`",
					"content": "Set the countdown, available `<template...>` variable are:\n* `{{remaining}}` - The remaining time in the countdown.\n* `{{elapsed}}` - The elapsed time in the countdown.\n* `{{duration}}` - The total duration of the countdown.\n",
					"examples": [
						{
							"name": "`setbac` setting a countdown of 5s 30s",
							"content": "setbac: !countdown set 5m30s Thing will happen in {{remaining}}\nSetMod: setbac -> Countdown set!\n"
						}
					]
				},
				{
					"name": "!countdown clear",
					"content": "Clear the current countdown.\n"
				}
			]
		},
		{
			"name": "Water reminders",
			"content": "A helper command to remind the streamer to drink water.\n",
			"commands": [
				{
					"name": "!water",
					"content": "Remind the streamer to drink water and receive a currency reward.\n",
					"examples": [
						{
							"name": "`bdogs_gaming` reminding `setbac` to drink some water",
							"content": "bdogs_gaming: !water\nSetMod: bdogs_gaming -> setbac, DRINK SOME WATER! bdogs_gaming has been rewarded 34 ether for the reminder.\n"
						}
					]
				},
				{
					"name": "!water undo",
					"content": "Undo the last water command.\n",
					"examples": [
						{
							"name": "`setbac` undoing `bdogs_gaming`'s water command",
							"content": "setbac: !water udno\nbdogs_gaming issued a bad !water that is now being undone FeelsBadMan\n"
						}
					]
				}
			]
		},
		{
			"name": "Promotions",
			"content": "Set promotions which will run at a periodic interval in chat.\n",
			"commands": [
				{
					"name": "!promo list",
					"content": "List all available promotions."
				},
				{
					"name": "!promo edit `<id>` `<frequency>` `<what>`",
					"content": "Set the promotion identified by `<id>` to send the message `<what>` every `<frequency>`.\n\n`<frequency>` has to be formatted as `[<days>d][<hours>h][<minutes>m][<seconds>s]`, like _5d10m30s_ or _5m_.\n",
					"examples": [
						{
							"name": "Set a promition for your Discord",
							"content": "setbac: !promo edit discord 30m Want to provide suggestions for future features? You can !afterstream me or join my Discord at https://discord.gg/v5AeNkT\nSetMod: setbac -> Edited promo.\n"
						}
					]
				},
				{
					"name": "!promo clear-group `<name>`",
					"content": "Clear the group for promotion `<name>`."
				},
				{
					"name": "!promo group `<name>`",
					"content": "Get the group the given promotion belongs to."
				},
				{
					"name": "!promo group `<name>` `<group>`",
					"content": "Set the promotion `<name>` to be in the group `<group>`."
				},
				{
					"name": "!promo delete `<name>`",
					"content": "Delete the promotion with the given `<name>`."
				},
				{
					"name": "!promo rename `<from>` `<to>`",
					"content": "Rename promotion `<from>` to `<to>`."
				}
			]
		},
		{
			"name": "Theme Commands",
			"content": "These are commands which administrate available theme songs.\n\nTheme songs are songs which can be played instantly through the player using: !song theme `<name>`\n",
			"commands": [
				{
					"name": "!theme list",
					"content": "List all available themes."
				},
				{
					"name": "!theme edit `<id>` `<track-uri>`",
					"content": "Set the theme identified by `<id>` to play the track `<track-uri>`.",
					"examples": [
						{
							"name": "Set the theme to a Spotify Song",
							"content": "setbac: !theme edit setup spotify:track:2fZpKgrcAlWzWYwQeFG43O\nSetMod: setbac -> Edited theme.\n"
						},
						{
							"name": "Set the theme to a YouTube Song",
							"content": "setbac: !theme edit ayaya youtube:video:D0q0QeQbw9U\nSetMod: setbac -> Edited theme.\n"
						}
					]
				},
				{
					"name": "!theme edit-duration `<start>` `[end]`",
					"content": "Set the playback duration of the theme from `<start>` up until an optional `[end]`.\n\nIf no `[end]` is specific, the theme will play until the end of the song.\n",
					"examples": [
						{
							"name": "Set the duration of the theme `setup` to start at _10 seconds_ and end at `01:10`",
							"content": "setbac: !theme edit-duration setup 00:10 01:10\nSetMod: setbac -> Edited theme.\n"
						}
					]
				},
				{
					"name": "!theme clear-group `<name>`",
					"content": "Clear the group for theme `<name>`."
				},
				{
					"name": "!theme group `<name>`",
					"content": "Get the group the given theme belongs to."
				},
				{
					"name": "!theme group `<name>` `<group>`",
					"content": "Set the theme `<name>` to be in the group `<group>`."
				},
				{
					"name": "!theme delete `<id>`",
					"content": "Delete the theme with the given `<id>`."
				},
				{
					"name": "!theme rename `<from>` `<to>`",
					"content": "Rename the theme `<from>` to `<to>`."
				}
			]
		},
		{
			"name": "Time Commands",
			"content": "Commands related to dealing with the current time.\n",
			"commands": [
				{
					"name": "!time",
					"content": "Show the current, _configured_ time for the streamer.",
					"examples": [
						{
							"name": "Showing the current time for the streamer",
							"content": "setbac: !time\nSetMod: setbac -> The time in Stockholm, Sweden is 15:21:57+0200\n"
						}
					]
				}
			]
		},
		{
			"name": "Polling Commands",
			"content": "These are commands related to running in-chat polls.\n",
			"commands": [
				{
					"name": "!poll run `<question>` `<options...>`",
					"content": "Run the poll with the given `<question>`, providing the options listed in `<options...>`.",
					"examples": [
						{
							"name": "Streamer running a poll for which game to play",
							"content": "setbac: !poll run \"Which game should I play?\" 1=\"GTA 5\" 2=\"GTA SA\" 3=\"don't care\"\nSetMod: setbac -> Started poll `Which game should I play?`\nturtle: 2\nSetMod: Now playing: \"The Veldt - Radio Edit\" by deadmau5.\nhare: 1\nsetbac: !poll close\nSetMod: setbac -> Which game should I play? -> GTA SA = one vote (50%), GTA 5 = one vote (50%), don't care = no votes (0%).\n"
						}
					]
				}
			]
		},
		{
			"name": "Weather Commands",
			"content": "Commands for getting the weather at a specific location.\n",
			"commands": [
				{
					"name": "!weather current",
					"content": "Get the current weather at the streamer's location",
					"examples": [
						{
							"name": "Getting the current weather at the streamer's location",
							"content": "setbac: !weather current\nSetMod: setbac -> Stockholm -> 7.9 °C, shower rain 🌧️.\n"
						}
					]
				},
				{
					"name": "!weather current `<location...>`",
					"content": "Get the current weather at the specified `<location...>`.",
					"examples": [
						{
							"name": "Getting the current weather at the specified location",
							"content": "setbac: !weather current Moscow\nSetMod: setbac -> Moscow -> 3.2 °C, overcast clouds 🌧️.\n"
						}
					]
				}
			]
		},
		{
			"name": "speedrun.com integration",
			"content": "Commands integrating with speedrun.com\n",
			"commands": [
				{
					"name": "!speedrun game `<game>` `[filters]`",
					"content": "Get the record for a specific `<game>`.\n\nAvailable `[filters]` are:\n* `--user <name>` - Limit results to the given user.\n* `--abbrev` - Abbreviate sub-categories (e.g. \"100% No Mission Skips\" becomes \"100% NMS\").\n* `--category <name>` - Limit results to the given category.\n* `--sub-category <name>` - Limit results to the given sub-category.\n* `--misc` - Include misc categories.\n* `--misc-only` - Only list misc categories.\n",
					"examples": [
						{
							"name": "Get the record for the 100% categories of GTA V",
							"content": "setbac: !speedrun game gtav --category 100%\nSetMod: setbac -> 100% (No Mission Skips) -> burhác: 9h 49m 44s (#1) | 100% (Mission Skips) -> Reloe: 8h 17m 39s (#1)\n"
						}
					]
				},
				{
					"name": "!speedrun personal-bests `<user>` `[filters]`",
					"content": "Get all personal bests for the specified `<user>`.\n\nAvailable `[filters]` are:\n* `--game <game>` - Limit results to the given game.\n* `--abbrev` - Abbreviate sub-categories (e.g. \"100% No Mission Skips\" becomes \"100% NMS\").\n* `--per-level` - Show per-level personal bests.\n* `--level <level>` - Filter by the given level.\n* `--category <name>` - Limit results to the given category.\n* `--sub-category <name>` - Limit results to the given sub-category.\n* `--misc` - Include misc categories.\n* `--misc-only` - Only list misc categories.\n",
					"examples": [
						{
							"name": "Get the personal bests for `setbac` in GTA V",
							"content": "setbac: !speedrun personal-bests setbac --game gtav\nSetMod: setbac -> Grand Theft Auto V (gtav) -> Classic%: 6h 44m 4s (#11)\n"
						}
					]
				}
			]
		},
		{
			"name": "!auth",
			"content": "Commands for listing your scopes and granting scopes to others.\n",
			"commands": [
				{
					"name": "!auth scopes `[filter]`",
					"content": "List your scopes with an optional `[filter]`.\n",
					"examples": [
						{
							"name": "Get your scopes containing `song`",
							"content": "setbac: !auth scopes song\nSetMod: setbac -> @moderator: song/playback-control, song/spotify, song/list-limit, song/edit-queue, song/volume, song/theme | @subscriber: song/spotify | @everyone: song\n"
						}
					]
				},
				{
					"name": "!auth permit `<duration>` `<principal>` `<scope>`",
					"content": "Grant a `<scope>` to `<principal>` for `<duration>`.\n\n`<principal>` can either be a role (e.g. _@everyone_ or _@subscriber_) or a user (without _@_).\n`<duration>` has to be formatted as `[<days>d][<hours>h][<minutes>m][<seconds>s]`, like _5d10m30s_ or _5m_.\n",
					"examples": [
						{
							"name": "Grant _song/spotify_ to _user123_ for _1 minute_",
							"content": "setbac: !auth permit 1m user123 song/spotify\nSetMod: setbac -> Gave: song/spotify to user123 for 1m\n"
						}
					]
				}
			]
		}
	]
}

/***/ }),

/***/ 6469:
/***/ ((__unused_webpack_module, __unused_webpack___webpack_exports__, __webpack_require__) => {

"use strict";

// NAMESPACE OBJECT: ./src/assets/enable-remote-updates.png
var enable_remote_updates_namespaceObject = {};
__webpack_require__.r(enable_remote_updates_namespaceObject);
__webpack_require__.d(enable_remote_updates_namespaceObject, {
  "default": () => (enable_remote_updates)
});

// EXTERNAL MODULE: ./node_modules/react/index.js
var react = __webpack_require__(7294);
// EXTERNAL MODULE: ./node_modules/react-dom/index.js
var react_dom = __webpack_require__(3935);
// EXTERNAL MODULE: ./node_modules/react-router-dom/esm/react-router-dom.js
var react_router_dom = __webpack_require__(3727);
// EXTERNAL MODULE: ./node_modules/react-router/esm/react-router.js + 1 modules
var react_router = __webpack_require__(5977);
// EXTERNAL MODULE: ./node_modules/regenerator-runtime/runtime.js
var runtime = __webpack_require__(5666);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.concat.js
var es_array_concat = __webpack_require__(2222);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.promise.js
var es_promise = __webpack_require__(8674);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.to-string.js
var es_object_to_string = __webpack_require__(1539);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.iterator.js
var es_string_iterator = __webpack_require__(8783);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.iterator.js
var es_array_iterator = __webpack_require__(6992);
// EXTERNAL MODULE: ./node_modules/core-js/modules/web.dom-collections.iterator.js
var web_dom_collections_iterator = __webpack_require__(3948);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.starts-with.js
var es_string_starts_with = __webpack_require__(6755);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.symbol.js
var es_symbol = __webpack_require__(2526);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.symbol.description.js
var es_symbol_description = __webpack_require__(1817);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.symbol.iterator.js
var es_symbol_iterator = __webpack_require__(2165);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.slice.js
var es_array_slice = __webpack_require__(7042);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.function.name.js
var es_function_name = __webpack_require__(8309);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.from.js
var es_array_from = __webpack_require__(1038);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.regexp.to-string.js
var es_regexp_to_string = __webpack_require__(9714);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.join.js
var es_array_join = __webpack_require__(9600);
;// CONCATENATED MODULE: ./src/api.js
function _createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = _unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function _unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return _arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _arrayLikeToArray(o, minLen); }

function _arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }



function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function _asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }
















function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function _defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function _createClass(Constructor, protoProps, staticProps) { if (protoProps) _defineProperties(Constructor.prototype, protoProps); if (staticProps) _defineProperties(Constructor, staticProps); return Constructor; }

var ApiError = /*#__PURE__*/function () {
  function ApiError(status, body) {
    _classCallCheck(this, ApiError);

    this.status = status;
    this.body = body;
  }
  /**
   * Test if the error is a 404 not found.
   */


  _createClass(ApiError, [{
    key: "notFound",
    value: function notFound() {
      return this.status == 404;
    }
  }, {
    key: "toString",
    value: function toString() {
      return "got bad status code ".concat(this.status, ": ").concat(this.body);
    }
  }]);

  return ApiError;
}();
var Api = /*#__PURE__*/function () {
  function Api(url) {
    _classCallCheck(this, Api);

    this.url = url;
  }
  /**
   *
   * @param {string | array<string>} path
   * @param {*} data
   */


  _createClass(Api, [{
    key: "fetch",
    value: function (_fetch) {
      function fetch(_x) {
        return _fetch.apply(this, arguments);
      }

      fetch.toString = function () {
        return _fetch.toString();
      };

      return fetch;
    }(
    /*#__PURE__*/
    function () {
      var _ref = _asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee(path) {
        var data,
            r,
            text,
            _args = arguments;
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                data = _args.length > 1 && _args[1] !== undefined ? _args[1] : {};

                if (path instanceof Array) {
                  path = encodePath(path);
                }

                data.credentials = "same-origin";
                _context.next = 5;
                return fetch("".concat(this.url, "/").concat(path), data);

              case 5:
                r = _context.sent;

                if (r.ok) {
                  _context.next = 11;
                  break;
                }

                _context.next = 9;
                return r.text();

              case 9:
                text = _context.sent;
                throw new ApiError(r.status, text);

              case 11:
                _context.next = 13;
                return r.json();

              case 13:
                return _context.abrupt("return", _context.sent);

              case 14:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      return function (_x2) {
        return _ref.apply(this, arguments);
      };
    }()
    /**
     * Get list of players.
     */
    )
  }, {
    key: "players",
    value: function players() {
      return this.fetch(["players"]);
    }
    /**
     * Get information about the specified player.
     */

  }, {
    key: "player",
    value: function player(id) {
      return this.fetch(["player", id]);
    }
    /**
     * Login the current user.
     */

  }, {
    key: "authLogin",
    value: function authLogin() {
      return this.fetch(["auth", "login"], {
        method: "POST"
      });
    }
    /**
     * Logout the current user.
     */

  }, {
    key: "authLogout",
    value: function authLogout() {
      return this.fetch(["auth", "logout"], {
        method: "POST"
      });
    }
    /**
     * Get information on the current user.
     */

  }, {
    key: "authCurrent",
    value: function authCurrent() {
      return this.fetch(["auth", "current"]);
    }
    /**
     * List all available connections.
     */

  }, {
    key: "connectionsList",
    value: function connectionsList() {
      return this.fetch(["connections"]);
    }
    /**
     * Remove the given connection.
     */

  }, {
    key: "connectionsRemove",
    value: function connectionsRemove(id) {
      return this.fetch(["connections", id], {
        method: "DELETE"
      });
    }
    /**
     * Prepare to create the given connection.
     */

  }, {
    key: "connectionsCreate",
    value: function connectionsCreate(id) {
      return this.fetch(["connections", id], {
        method: "POST"
      });
    }
    /**
     * Get a list of all available connection types.
     */

  }, {
    key: "connectionTypes",
    value: function connectionTypes() {
      return this.fetch(["connection-types"]);
    }
    /**
     * Create a new key.
     */

  }, {
    key: "createKey",
    value: function createKey() {
      return this.fetch(["key"], {
        method: "POST"
      });
    }
    /**
     * Delete the current key.
     */

  }, {
    key: "deleteKey",
    value: function deleteKey() {
      return this.fetch(["key"], {
        method: "DELETE"
      });
    }
    /**
     * Get the current key.
     */

  }, {
    key: "getKey",
    value: function getKey() {
      return this.fetch(["key"]);
    }
  }, {
    key: "githubReleases",
    value: function githubReleases(user, repo) {
      return this.fetch(["github-releases", user, repo]);
    }
  }]);

  return Api;
}();

function encodePath(path) {
  var out = [];

  var _iterator = _createForOfIteratorHelper(path),
      _step;

  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var part = _step.value;
      out.push(encodeURIComponent(part));
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }

  return out.join("/");
}
;// CONCATENATED MODULE: ./src/globals.js
function _slicedToArray(arr, i) { return _arrayWithHoles(arr) || _iterableToArrayLimit(arr, i) || globals_unsupportedIterableToArray(arr, i) || _nonIterableRest(); }

function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }

function globals_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return globals_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return globals_arrayLikeToArray(o, minLen); }

function globals_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

function _iterableToArrayLimit(arr, i) { if (typeof Symbol === "undefined" || !(Symbol.iterator in Object(arr))) return; var _arr = []; var _n = true; var _d = false; var _e = undefined; try { for (var _i = arr[Symbol.iterator](), _s; !(_n = (_s = _i.next()).done); _n = true) { _arr.push(_s.value); if (i && _arr.length === i) break; } } catch (err) { _d = true; _e = err; } finally { try { if (!_n && _i["return"] != null) _i["return"](); } finally { if (_d) throw _e; } } return _arr; }

function _arrayWithHoles(arr) { if (Array.isArray(arr)) return arr; }
















function globals_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function globals_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { globals_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { globals_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }


var api = new Api(apiUrl());
var currentUser = null;
var currentConnections = [];
var cameFromBot = null;

function authCurrent() {
  return _authCurrent.apply(this, arguments);
}

function _authCurrent() {
  _authCurrent = globals_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
    return regeneratorRuntime.wrap(function _callee$(_context) {
      while (1) {
        switch (_context.prev = _context.next) {
          case 0:
            _context.prev = 0;
            _context.next = 3;
            return api.authCurrent();

          case 3:
            return _context.abrupt("return", _context.sent);

          case 6:
            _context.prev = 6;
            _context.t0 = _context["catch"](0);
            return _context.abrupt("return", null);

          case 9:
          case "end":
            return _context.stop();
        }
      }
    }, _callee, null, [[0, 6]]);
  }));
  return _authCurrent.apply(this, arguments);
}

function connectionTypes() {
  return _connectionTypes.apply(this, arguments);
}

function _connectionTypes() {
  _connectionTypes = globals_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
    return regeneratorRuntime.wrap(function _callee2$(_context2) {
      while (1) {
        switch (_context2.prev = _context2.next) {
          case 0:
            _context2.prev = 0;
            _context2.next = 3;
            return api.connectionTypes();

          case 3:
            return _context2.abrupt("return", _context2.sent);

          case 6:
            _context2.prev = 6;
            _context2.t0 = _context2["catch"](0);
            return _context2.abrupt("return", []);

          case 9:
          case "end":
            return _context2.stop();
        }
      }
    }, _callee2, null, [[0, 6]]);
  }));
  return _connectionTypes.apply(this, arguments);
}

function updateGlobals() {
  return _updateGlobals.apply(this, arguments);
}
/**
 * Get the current URL to connect to.
 */

function _updateGlobals() {
  _updateGlobals = globals_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3() {
    var _yield$Promise$all, _yield$Promise$all2, user, connections;

    return regeneratorRuntime.wrap(function _callee3$(_context3) {
      while (1) {
        switch (_context3.prev = _context3.next) {
          case 0:
            _context3.next = 2;
            return Promise.all([authCurrent(), connectionTypes()]);

          case 2:
            _yield$Promise$all = _context3.sent;
            _yield$Promise$all2 = _slicedToArray(_yield$Promise$all, 2);
            user = _yield$Promise$all2[0];
            connections = _yield$Promise$all2[1];
            currentUser = user;
            currentConnections = connections;

            if (document.referrer.startsWith("http://localhost")) {
              cameFromBot = document.referrer;
            }

          case 9:
          case "end":
            return _context3.stop();
        }
      }
    }, _callee3);
  }));
  return _updateGlobals.apply(this, arguments);
}

function apiUrl() {
  var loc = window.location;
  var scheme = "http";

  if (loc.protocol === "https:") {
    scheme = "https";
  }

  return "".concat(scheme, "://").concat(loc.host, "/api");
}
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.match.js
var es_string_match = __webpack_require__(4723);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.regexp.exec.js
var es_regexp_exec = __webpack_require__(4916);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.filter.js
var es_array_filter = __webpack_require__(7327);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.map.js
var es_array_map = __webpack_require__(1249);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.ends-with.js
var es_string_ends_with = __webpack_require__(7852);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.flat-map.js
var es_array_flat_map = __webpack_require__(6535);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.unscopables.flat-map.js
var es_array_unscopables_flat_map = __webpack_require__(9244);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.set-prototype-of.js
var es_object_set_prototype_of = __webpack_require__(8304);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.get-prototype-of.js
var es_object_get_prototype_of = __webpack_require__(489);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.reflect.construct.js
var es_reflect_construct = __webpack_require__(2419);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.keys.js
var es_object_keys = __webpack_require__(7941);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Card.js + 1 modules
var Card = __webpack_require__(5881);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Row.js
var Row = __webpack_require__(4051);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Col.js
var Col = __webpack_require__(1555);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/CardDeck.js
var CardDeck = __webpack_require__(4415);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Nav.js + 5 modules
var Nav = __webpack_require__(4456);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Navbar.js + 4 modules
var Navbar = __webpack_require__(103);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Container.js
var Container = __webpack_require__(682);
// EXTERNAL MODULE: ./node_modules/@fortawesome/react-fontawesome/index.es.js
var index_es = __webpack_require__(7814);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Button.js
var Button = __webpack_require__(7104);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Form.js + 10 modules
var Form = __webpack_require__(2258);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/ButtonGroup.js
var ButtonGroup = __webpack_require__(2086);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Dropdown.js + 74 modules
var Dropdown = __webpack_require__(8693);
;// CONCATENATED MODULE: ./src/assets/twitch.png
/* harmony default export */ const twitch = (__webpack_require__.p + "c6bdd2f06ae292a32e75c99a55f4b024.png");
;// CONCATENATED MODULE: ./src/assets/logo.png
/* harmony default export */ const logo = (__webpack_require__.p + "d1c48652deb589dccee97a958403d081.png");
;// CONCATENATED MODULE: ./src/components/CurrentUser.js
function _typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { _typeof = function _typeof(obj) { return typeof obj; }; } else { _typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return _typeof(obj); }














function CurrentUser_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function CurrentUser_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { CurrentUser_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { CurrentUser_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function CurrentUser_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function CurrentUser_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function CurrentUser_createClass(Constructor, protoProps, staticProps) { if (protoProps) CurrentUser_defineProperties(Constructor.prototype, protoProps); if (staticProps) CurrentUser_defineProperties(Constructor, staticProps); return Constructor; }

function _inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) _setPrototypeOf(subClass, superClass); }

function _setPrototypeOf(o, p) { _setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return _setPrototypeOf(o, p); }

function _createSuper(Derived) { var hasNativeReflectConstruct = _isNativeReflectConstruct(); return function _createSuperInternal() { var Super = _getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = _getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return _possibleConstructorReturn(this, result); }; }

function _possibleConstructorReturn(self, call) { if (call && (_typeof(call) === "object" || typeof call === "function")) { return call; } return _assertThisInitialized(self); }

function _assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function _isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function _getPrototypeOf(o) { _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return _getPrototypeOf(o); }









var CurrentUser = /*#__PURE__*/function (_React$Component) {
  _inherits(CurrentUser, _React$Component);

  var _super = _createSuper(CurrentUser);

  function CurrentUser(props) {
    CurrentUser_classCallCheck(this, CurrentUser);

    return _super.call(this, props);
  }

  CurrentUser_createClass(CurrentUser, [{
    key: "login",
    value: function () {
      var _login = CurrentUser_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        var result;
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return api.authLogin();

              case 2:
                result = _context.sent;
                location.href = result.auth_url;

              case 4:
              case "end":
                return _context.stop();
            }
          }
        }, _callee);
      }));

      function login() {
        return _login.apply(this, arguments);
      }

      return login;
    }()
  }, {
    key: "logout",
    value: function () {
      var _logout = CurrentUser_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var result;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                _context2.next = 2;
                return api.authLogout();

              case 2:
                result = _context2.sent;
                location.reload();

              case 4:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2);
      }));

      function logout() {
        return _logout.apply(this, arguments);
      }

      return logout;
    }()
  }, {
    key: "backToBot",
    value: function backToBot() {
      document.location.href = cameFromBot;
    }
  }, {
    key: "render",
    value: function render() {
      var _this = this;

      var backLink = null;

      if (cameFromBot !== null) {
        backLink = /*#__PURE__*/react.createElement(Button/* default */.Z, {
          variant: "warning",
          size: "sm",
          onClick: function onClick() {
            return _this.backToBot();
          },
          title: "Go back to your local OxidizeBot instance"
        }, "Back to ", /*#__PURE__*/react.createElement("img", {
          src: logo,
          width: "18",
          height: "18"
        }));
      }

      var button = /*#__PURE__*/react.createElement(Form/* default */.Z, {
        inline: true,
        key: "second"
      }, /*#__PURE__*/react.createElement(ButtonGroup/* default */.Z, null, backLink, /*#__PURE__*/react.createElement(Button/* default */.Z, {
        size: "sm",
        onClick: this.login.bind(this),
        title: "Sign in through Twitch"
      }, /*#__PURE__*/react.createElement("b", null, "Sign in with"), "\xA0", /*#__PURE__*/react.createElement("img", {
        src: twitch,
        height: "16px",
        width: "48px",
        alt: "twitch"
      }))));

      if (currentUser) {
        button = /*#__PURE__*/react.createElement(Dropdown/* default */.Z, {
          key: "second"
        }, /*#__PURE__*/react.createElement(ButtonGroup/* default */.Z, null, backLink, /*#__PURE__*/react.createElement(Dropdown/* default.Toggle */.Z.Toggle, {
          size: "sm"
        }, "Signed in: ", /*#__PURE__*/react.createElement("b", null, currentUser.login))), /*#__PURE__*/react.createElement(Dropdown/* default.Menu */.Z.Menu, null, /*#__PURE__*/react.createElement(Dropdown/* default.Item */.Z.Item, {
          as: react_router_dom/* Link */.rU,
          to: "/connections"
        }, "My Connections"), /*#__PURE__*/react.createElement(Dropdown/* default.Divider */.Z.Divider, null), /*#__PURE__*/react.createElement(Dropdown/* default.Item */.Z.Item, {
          onClick: this.logout.bind(this)
        }, "Sign out ", /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
          icon: "sign-out-alt"
        }))));
      }

      return button;
    }
  }]);

  return CurrentUser;
}(react.Component);


;// CONCATENATED MODULE: ./src/assets/logo-32px.png
/* harmony default export */ const logo_32px = (__webpack_require__.p + "73e38697bca0dc85c18844d844c141f3.png");
;// CONCATENATED MODULE: ./src/components/Layout.js
function Layout_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Layout_typeof = function _typeof(obj) { return typeof obj; }; } else { Layout_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Layout_typeof(obj); }

function Layout_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Layout_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Layout_createClass(Constructor, protoProps, staticProps) { if (protoProps) Layout_defineProperties(Constructor.prototype, protoProps); if (staticProps) Layout_defineProperties(Constructor, staticProps); return Constructor; }

function Layout_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Layout_setPrototypeOf(subClass, superClass); }

function Layout_setPrototypeOf(o, p) { Layout_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Layout_setPrototypeOf(o, p); }

function Layout_createSuper(Derived) { var hasNativeReflectConstruct = Layout_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Layout_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Layout_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Layout_possibleConstructorReturn(this, result); }; }

function Layout_possibleConstructorReturn(self, call) { if (call && (Layout_typeof(call) === "object" || typeof call === "function")) { return call; } return Layout_assertThisInitialized(self); }

function Layout_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Layout_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Layout_getPrototypeOf(o) { Layout_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Layout_getPrototypeOf(o); }





















function links(props) {
  var links = [];
  links.push( /*#__PURE__*/react.createElement(Nav/* default.Item */.Z.Item, {
    key: "help"
  }, /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
    as: react_router_dom/* Link */.rU,
    active: props.match.path === "/help",
    to: "/help"
  }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
    icon: "question"
  }), "\xA0Help")));
  links.push( /*#__PURE__*/react.createElement(Nav/* default.Item */.Z.Item, {
    key: "connections"
  }, /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
    as: react_router_dom/* Link */.rU,
    active: props.match.path === "/connections",
    to: "/connections"
  }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
    icon: "globe"
  }), "\xA0My\xA0Connections")));
  links.push( /*#__PURE__*/react.createElement(Nav/* default.Item */.Z.Item, {
    key: "playlists"
  }, /*#__PURE__*/react.createElement(Nav/* default.Link */.Z.Link, {
    as: react_router_dom/* Link */.rU,
    active: props.match.path === "/playlists" || props.match.path === "/player/:id",
    to: "/playlists"
  }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
    icon: "music"
  }), "\xA0Playlists")));
  return links;
}

var Layout = /*#__PURE__*/function (_React$Component) {
  Layout_inherits(Layout, _React$Component);

  var _super = Layout_createSuper(Layout);

  function Layout(props) {
    Layout_classCallCheck(this, Layout);

    return _super.call(this, props);
  }

  Layout_createClass(Layout, [{
    key: "render",
    value: function render() {
      var navLinks = links(this.props);
      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("div", {
        key: "navigation",
        id: "navbar"
      }, /*#__PURE__*/react.createElement(Navbar/* default */.Z, {
        key: "nav",
        expand: "sm",
        className: "mb-3",
        bg: "light"
      }, /*#__PURE__*/react.createElement(Container/* default */.Z, null, /*#__PURE__*/react.createElement(Navbar/* default.Brand */.Z.Brand, null, /*#__PURE__*/react.createElement(react_router_dom/* Link */.rU, {
        to: "/"
      }, /*#__PURE__*/react.createElement("img", {
        src: logo_32px,
        alt: "Logo",
        width: "32",
        height: "32"
      }))), /*#__PURE__*/react.createElement(Navbar/* default.Collapse */.Z.Collapse, null, /*#__PURE__*/react.createElement(Nav/* default */.Z, null, navLinks), /*#__PURE__*/react.createElement(Nav/* default */.Z, {
        className: "ml-auto"
      }, /*#__PURE__*/react.createElement(Nav/* default.Item */.Z.Item, {
        className: "nav-link"
      }, /*#__PURE__*/react.createElement(CurrentUser, null)))), /*#__PURE__*/react.createElement(Navbar/* default.Toggle */.Z.Toggle, {
        "aria-controls": "basic-navbar-nav"
      })))), /*#__PURE__*/react.createElement(Container/* default */.Z, {
        key: "content",
        id: "content",
        className: "mb-3"
      }, this.props.children), /*#__PURE__*/react.createElement(Container/* default */.Z, {
        key: "footer",
        id: "footer",
        className: "pt-2 pb-2"
      }, /*#__PURE__*/react.createElement("span", {
        className: "oxi-highlight"
      }, "setbac.tv"), " is built and operated with \u2665 by ", /*#__PURE__*/react.createElement("a", {
        href: "https://twitch.tv/setbac"
      }, "setbac"), " (", /*#__PURE__*/react.createElement("a", {
        href: "https://github.com/udoprog",
        title: "Github"
      }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
        icon: ['fab', 'github']
      })), " - ", /*#__PURE__*/react.createElement("a", {
        href: "https://twitter.com/udoprog",
        title: "Twitter"
      }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
        icon: ['fab', 'twitter']
      })), " - ", /*#__PURE__*/react.createElement("a", {
        href: "https://twitch.com/setbac"
      }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
        icon: ['fab', 'twitch'],
        title: "Twitch"
      })), ")", /*#__PURE__*/react.createElement("br", null), "Come join my ", /*#__PURE__*/react.createElement("a", {
        href: "https://discord.gg/v5AeNkT"
      }, "Discord Community"), " if you want to participate in this Project", /*#__PURE__*/react.createElement("br", null), /*#__PURE__*/react.createElement(react_router_dom/* Link */.rU, {
        to: "/"
      }, "Start Page"), " \u2013 ", /*#__PURE__*/react.createElement(react_router_dom/* Link */.rU, {
        to: "/privacy"
      }, "Privacy Policy")));
    }
  }]);

  return Layout;
}(react.Component);

var RouteLayout = (0,react_router/* withRouter */.EN)(function (props) {
  return /*#__PURE__*/react.createElement(Layout, props);
});
;// CONCATENATED MODULE: ./src/assets/twitch-dark.png
/* harmony default export */ const twitch_dark = (__webpack_require__.p + "592f04a6992755a530e6553e594ade37.png");
;// CONCATENATED MODULE: ./src/assets/windows.svg
/* harmony default export */ const windows = (__webpack_require__.p + "8b0e645f6ede3816f4bc94be9fd535a2.svg");
;// CONCATENATED MODULE: ./src/assets/debian.svg
/* harmony default export */ const debian = (__webpack_require__.p + "429174d06fdb61f04367fa7fbda3979d.svg");
;// CONCATENATED MODULE: ./src/assets/mac.svg
/* harmony default export */ const mac = (__webpack_require__.p + "fba885682df03157d7919b6d43f21c06.svg");
// EXTERNAL MODULE: ./node_modules/react-inlinesvg/esm/index.js + 3 modules
var esm = __webpack_require__(4934);
// EXTERNAL MODULE: ../shared-ui/node_modules/react/index.js
var node_modules_react = __webpack_require__(2321);
;// CONCATENATED MODULE: ../shared-ui/components/Loading.js

function Loading(props) {
  if (props.isLoading !== undefined && !props.isLoading) {
    return null;
  }

  var info = null;

  if (props.children) {
    info = /*#__PURE__*/node_modules_react.createElement("div", {
      className: "oxi-loading-info"
    }, props.children);
  }

  return /*#__PURE__*/node_modules_react.createElement("div", {
    className: "oxi-loading"
  }, info, /*#__PURE__*/node_modules_react.createElement("div", {
    className: "spinner-border",
    role: "status"
  }, /*#__PURE__*/node_modules_react.createElement("span", {
    className: "sr-only"
  }, "Loading...")));
}
;// CONCATENATED MODULE: ./src/components/Index.js
function Index_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Index_typeof = function _typeof(obj) { return typeof obj; }; } else { Index_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Index_typeof(obj); }

function _toConsumableArray(arr) { return _arrayWithoutHoles(arr) || _iterableToArray(arr) || Index_unsupportedIterableToArray(arr) || _nonIterableSpread(); }

function _nonIterableSpread() { throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }

function _iterableToArray(iter) { if (typeof Symbol !== "undefined" && Symbol.iterator in Object(iter)) return Array.from(iter); }

function _arrayWithoutHoles(arr) { if (Array.isArray(arr)) return Index_arrayLikeToArray(arr); }

function _objectWithoutProperties(source, excluded) { if (source == null) return {}; var target = _objectWithoutPropertiesLoose(source, excluded); var key, i; if (Object.getOwnPropertySymbols) { var sourceSymbolKeys = Object.getOwnPropertySymbols(source); for (i = 0; i < sourceSymbolKeys.length; i++) { key = sourceSymbolKeys[i]; if (excluded.indexOf(key) >= 0) continue; if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue; target[key] = source[key]; } } return target; }

function _objectWithoutPropertiesLoose(source, excluded) { if (source == null) return {}; var target = {}; var sourceKeys = Object.keys(source); var key, i; for (i = 0; i < sourceKeys.length; i++) { key = sourceKeys[i]; if (excluded.indexOf(key) >= 0) continue; target[key] = source[key]; } return target; }



function Index_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Index_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Index_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Index_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Index_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Index_setPrototypeOf(subClass, superClass); }

function Index_setPrototypeOf(o, p) { Index_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Index_setPrototypeOf(o, p); }

function Index_createSuper(Derived) { var hasNativeReflectConstruct = Index_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Index_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Index_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Index_possibleConstructorReturn(this, result); }; }

function Index_possibleConstructorReturn(self, call) { if (call && (Index_typeof(call) === "object" || typeof call === "function")) { return call; } return Index_assertThisInitialized(self); }

function Index_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Index_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Index_getPrototypeOf(o) { Index_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Index_getPrototypeOf(o); }

function Index_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Index_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Index_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Index_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Index_arrayLikeToArray(o, minLen); }

function Index_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }

























function Index_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Index_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Index_createClass(Constructor, protoProps, staticProps) { if (protoProps) Index_defineProperties(Constructor.prototype, protoProps); if (staticProps) Index_defineProperties(Constructor, staticProps); return Constructor; }












var VERSION_REGEX = /(\d+)\.(\d+)\.(\d+)(-[a-z]+\.(\d+))?/;

var Version = /*#__PURE__*/function () {
  function Version(version) {
    Index_classCallCheck(this, Version);

    var out = version.match(VERSION_REGEX);

    if (!out) {
      throw new Error("Illegal Version: " + version);
    }

    this.parts = [parseInt(out[1]), parseInt(out[2]), parseInt(out[3]), Infinity];
    var prerelease = out[5];

    if (prerelease !== undefined) {
      this.parts[3] = parseInt(prerelease);
    }

    this.versionString = version;
  }

  Index_createClass(Version, [{
    key: "cmp",
    value: function cmp(o) {
      for (var i = 0; i < 4; i++) {
        if (this.parts[i] > o.parts[i]) {
          return 1;
        }

        if (this.parts[i] < o.parts[i]) {
          return -1;
        }
      }

      return 0;
    }
  }, {
    key: "toString",
    value: function toString() {
      return this.versionString;
    }
  }]);

  return Version;
}();
/**
 * Split releases into a stable and a prerelease.
 *
 * @param {*} releases
 */


function filterReleases(releases) {
  var stable = latestReleases(releases.filter(function (r) {
    return !r.prerelease;
  }), 2);
  var unstable = latestReleases(releases.filter(function (r) {
    return r.prerelease;
  }), 2);
  return {
    stable: stable,
    unstable: unstable
  };
}
/**
 * Get the latest release out of a collection of releases.
 *
 * @param {*} releases
 */


function latestReleases(releasesIn, n) {
  var releases = [];

  var _iterator = Index_createForOfIteratorHelper(releasesIn),
      _step;

  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var release = _step.value;

      try {
        releases.push({
          version: new Version(release.tag_name),
          release: release
        });
      } catch (e) {
        continue;
      }
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }

  releases.sort(function (a, b) {
    return b.version.cmp(a.version);
  });
  return releases.slice(0, Math.min(n, releases.length));
}

function partitionDownloads(incoming, unstable) {
  return incoming.map(function (_ref) {
    var release = _ref.release,
        version = _ref.version;
    var debian = [];
    var windows = [];
    var mac = [];

    var _iterator2 = Index_createForOfIteratorHelper(release.assets),
        _step2;

    try {
      for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
        var asset = _step2.value;

        if (asset.name.endsWith(".deb")) {
          debian.push({
            asset: asset,
            title: "Package",
            prio: 1
          });
          continue;
        }

        if (asset.name.endsWith(".msi")) {
          windows.push({
            asset: asset,
            title: "Installer",
            prio: 1
          });
          continue;
        }

        if (asset.name.endsWith(".zip")) {
          if (asset.name.indexOf("windows") != -1) {
            windows.push({
              asset: asset,
              title: "Zip Archive",
              prio: 0
            });
          } else if (asset.name.indexOf("linux") != -1) {
            debian.push({
              asset: asset,
              title: "Zip Archive",
              prio: 0
            });
          } else if (asset.name.indexOf("macos") != -1) {
            mac.push({
              asset: asset,
              title: "Zip Archive",
              prio: 0
            });
          }

          continue;
        }
      }
    } catch (err) {
      _iterator2.e(err);
    } finally {
      _iterator2.f();
    }

    return {
      version: version,
      unstable: unstable,
      debian: debian,
      windows: windows,
      mac: mac
    };
  });
}

var Index = /*#__PURE__*/function (_React$Component) {
  Index_inherits(Index, _React$Component);

  var _super = Index_createSuper(Index);

  function Index(props) {
    var _this;

    Index_classCallCheck(this, Index);

    _this = _super.call(this, props);
    _this.state = {
      releases: [],
      stable: null,
      unstable: null,
      loadingReleases: true
    };
    return _this;
  }
  /**
   * Refresh the known collection of releases.
   */


  Index_createClass(Index, [{
    key: "refreshReleases",
    value: function () {
      var _refreshReleases = Index_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        var releases, _filterReleases, stable, unstable;

        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                this.setState({
                  loadingReleases: true
                });
                _context.next = 3;
                return api.githubReleases('udoprog', 'OxidizeBot');

              case 3:
                releases = _context.sent;
                _filterReleases = filterReleases(releases), stable = _filterReleases.stable, unstable = _filterReleases.unstable;
                stable = partitionDownloads(stable, false);
                unstable = partitionDownloads(unstable, true);
                this.setState({
                  releases: releases,
                  stable: stable,
                  unstable: unstable,
                  loadingReleases: false
                });

              case 8:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function refreshReleases() {
        return _refreshReleases.apply(this, arguments);
      }

      return refreshReleases;
    }()
  }, {
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Index_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                _context2.next = 2;
                return this.refreshReleases();

              case 2:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
    /**
     * Optionally render download links in case they are available.
     */

  }, {
    key: "renderDownloadLinks",
    value: function renderDownloadLinks(data, filter) {
      return data.flatMap(function (_ref2) {
        var version = _ref2.version,
            unstable = _ref2.unstable,
            other = _objectWithoutProperties(_ref2, ["version", "unstable"]);

        return (filter(other) || []).map(function (_ref3) {
          var asset = _ref3.asset,
              title = _ref3.title,
              prio = _ref3.prio;
          var m = asset.name.match(/\.[a-z]+$/);
          var ext = null;

          if (m !== null) {
            ext = /*#__PURE__*/react.createElement(react.Fragment, null, " (", m[0], ")");
          }

          var unstableEl = null;

          if (unstable) {
            unstableEl = /*#__PURE__*/react.createElement(react.Fragment, null, " ", /*#__PURE__*/react.createElement("span", {
              className: "oxi-unstable",
              title: "Development version with new features, but has a higher risk of bugs"
            }, "DEV"));
          }

          var element = function element(key) {
            return /*#__PURE__*/react.createElement(Card/* default.Text */.Z.Text, {
              key: key
            }, /*#__PURE__*/react.createElement("a", {
              href: asset.browser_download_url
            }, /*#__PURE__*/react.createElement("b", null, version.toString()), " \u2013 ", title, ext), unstableEl);
          };

          return {
            element: element,
            version: version,
            prio: prio
          };
        });
      });
    }
  }, {
    key: "renderCard",
    value: function renderCard(filter, title, img) {
      var releases = [];

      if (!this.state.loadingReleases) {
        var _releases, _releases2;

        (_releases = releases).push.apply(_releases, _toConsumableArray(this.renderDownloadLinks(this.state.stable, filter)));

        (_releases2 = releases).push.apply(_releases2, _toConsumableArray(this.renderDownloadLinks(this.state.unstable, filter)));

        releases.sort(function (a, b) {
          var byVersion = b.version.cmp(a.version);

          if (byVersion === 0) {
            return b.prio - a.prio;
          }

          return byVersion;
        });
      }

      if (releases.length > 0) {
        releases = releases.map(function (r, i) {
          return r.element(i);
        });
      } else {
        releases = /*#__PURE__*/react.createElement(Card/* default.Text */.Z.Text, {
          key: "no-release",
          className: "oxi-center"
        }, "No Releases Yet!");
      }

      return /*#__PURE__*/react.createElement(Card/* default */.Z, null, /*#__PURE__*/react.createElement(Card/* default.Img */.Z.Img, {
        as: esm/* default */.Z,
        src: img,
        height: "80px",
        className: "mb-3 mt-3"
      }), /*#__PURE__*/react.createElement(Card/* default.Body */.Z.Body, null, /*#__PURE__*/react.createElement(Card/* default.Title */.Z.Title, {
        className: "oxi-center"
      }, title), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loadingReleases
      }), releases));
    }
  }, {
    key: "render",
    value: function render() {
      var windowsCard = this.renderCard(function (r) {
        return r.windows;
      }, "Windows", windows);
      var debianCard = this.renderCard(function (r) {
        return r.debian;
      }, "Debian", debian);
      var macCard = this.renderCard(function (r) {
        return r.mac;
      }, "Mac OS", mac);
      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement(Row/* default */.Z, {
        className: "oxi-intro"
      }, /*#__PURE__*/react.createElement(Col/* default */.Z, {
        sm: "8"
      }, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-title"
      }, "OxidizeBot"), /*#__PURE__*/react.createElement("p", null, "The high octane ", /*#__PURE__*/react.createElement("a", {
        href: "https://twitch.tv"
      }, /*#__PURE__*/react.createElement("img", {
        src: twitch_dark,
        height: "16px",
        width: "48px",
        alt: "twitch"
      })), " bot."), /*#__PURE__*/react.createElement("p", null, /*#__PURE__*/react.createElement("b", null, "OxidizeBot"), " as an open source Twitch Bot empowering you to focus on what's important."), /*#__PURE__*/react.createElement("p", null, "It allows for a richer interaction between you and your chat. From a song request system, to groundbreaking game modes where your viewers can interact directly with you and your game."), /*#__PURE__*/react.createElement("p", null, "It's written in ", /*#__PURE__*/react.createElement("a", {
        href: "https://rust-lang.org"
      }, "Rust"), ", providing an unparalleled level of reliability and performance.")), /*#__PURE__*/react.createElement(Col/* default */.Z, {
        sm: "4",
        className: "oxi-logo-big"
      }, /*#__PURE__*/react.createElement("img", {
        src: logo
      }))), /*#__PURE__*/react.createElement(CardDeck/* default */.Z, {
        className: "mb-4"
      }, /*#__PURE__*/react.createElement(Card/* default */.Z, null, /*#__PURE__*/react.createElement(Card/* default.Body */.Z.Body, null, /*#__PURE__*/react.createElement(Card/* default.Title */.Z.Title, {
        className: "oxi-center"
      }, /*#__PURE__*/react.createElement("b", null, "Free"), " and ", /*#__PURE__*/react.createElement("b", null, "Open Source")), /*#__PURE__*/react.createElement(Card/* default.Text */.Z.Text, null, "OxidizeBot doesn't cost you anything, and its source code is available on ", /*#__PURE__*/react.createElement("a", {
        href: "https://github.com/udoprog/OxidizeBot"
      }, "GitHub"), " for anyone to tinker with!"))), /*#__PURE__*/react.createElement(Card/* default */.Z, null, /*#__PURE__*/react.createElement(Card/* default.Body */.Z.Body, null, /*#__PURE__*/react.createElement(Card/* default.Title */.Z.Title, {
        className: "oxi-center"
      }, /*#__PURE__*/react.createElement("b", null, "Packed"), " with ", /*#__PURE__*/react.createElement("b", null, "Features")), /*#__PURE__*/react.createElement(Card/* default.Text */.Z.Text, null, "Plays music, moderates your chat, plays games, you name it!"), /*#__PURE__*/react.createElement(Card/* default.Text */.Z.Text, null, "If you feel something is missing, feel free to ", /*#__PURE__*/react.createElement("a", {
        href: "https://github.com/udoprog/OxidizeBot/issues"
      }, "open an issue"), "."))), /*#__PURE__*/react.createElement(Card/* default */.Z, null, /*#__PURE__*/react.createElement(Card/* default.Body */.Z.Body, null, /*#__PURE__*/react.createElement(Card/* default.Title */.Z.Title, {
        className: "oxi-center"
      }, "Runs on ", /*#__PURE__*/react.createElement("b", null, "Your Computer")), /*#__PURE__*/react.createElement(Card/* default.Text */.Z.Text, null, /*#__PURE__*/react.createElement("em", null, "You"), " own your data. It uses ", /*#__PURE__*/react.createElement("em", null, "your"), " internet for the best possible latency. It's light on system resources*. And running locally means it can perform rich interactions with your games like ", /*#__PURE__*/react.createElement("a", {
        href: "https://github.com/udoprog/ChaosMod"
      }, "Chaos%"), "."), /*#__PURE__*/react.createElement("div", {
        className: "oxi-subtext"
      }, "*: Low CPU usage and about 50MB of ram.")))), /*#__PURE__*/react.createElement("h4", {
        className: "oxi-center mb-4"
      }, "Downloads"), /*#__PURE__*/react.createElement(CardDeck/* default */.Z, null, windowsCard, debianCard, macCard));
    }
  }]);

  return Index;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Alert.js + 2 modules
var Alert = __webpack_require__(7953);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/Table.js
var Table = __webpack_require__(5147);
;// CONCATENATED MODULE: ./src/utils.js
var minute = 60,
    hour = minute * 60,
    day = hour * 24,
    week = day * 7;
/**
 * Get a human-readable duration since the given timestamp.
 */

function humanDurationSince(from) {
  from = new Date(from);
  var delta = Math.round((+new Date() - from) / 1000);
  var result;

  if (delta < 30) {
    result = 'Just now';
  } else if (delta < minute) {
    result = delta + ' seconds ago';
  } else if (delta < 2 * minute) {
    result = 'A minute ago';
  } else if (delta < hour) {
    result = Math.floor(delta / minute) + ' minutes ago';
  } else if (delta < hour * 2) {
    result = '1 hour ago';
  } else if (delta < day) {
    result = Math.floor(delta / hour) + ' hours ago';
  } else if (delta < day * 2) {
    result = 'Yesterday';
  } else if (delta < week) {
    result = Math.floor(delta / day) + ' days ago';
  } else if (delta < week * 2) {
    result = 'Last week';
  } else {
    result = Math.floor(delta / week) + ' weeks ago';
  }

  return result;
}
;// CONCATENATED MODULE: ./src/components/Playlists.js
function Playlists_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Playlists_typeof = function _typeof(obj) { return typeof obj; }; } else { Playlists_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Playlists_typeof(obj); }
















function Playlists_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Playlists_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Playlists_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Playlists_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Playlists_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Playlists_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Playlists_createClass(Constructor, protoProps, staticProps) { if (protoProps) Playlists_defineProperties(Constructor.prototype, protoProps); if (staticProps) Playlists_defineProperties(Constructor, staticProps); return Constructor; }

function Playlists_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Playlists_setPrototypeOf(subClass, superClass); }

function Playlists_setPrototypeOf(o, p) { Playlists_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Playlists_setPrototypeOf(o, p); }

function Playlists_createSuper(Derived) { var hasNativeReflectConstruct = Playlists_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Playlists_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Playlists_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Playlists_possibleConstructorReturn(this, result); }; }

function Playlists_possibleConstructorReturn(self, call) { if (call && (Playlists_typeof(call) === "object" || typeof call === "function")) { return call; } return Playlists_assertThisInitialized(self); }

function Playlists_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Playlists_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Playlists_getPrototypeOf(o) { Playlists_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Playlists_getPrototypeOf(o); }









var Players = /*#__PURE__*/function (_React$Component) {
  Playlists_inherits(Players, _React$Component);

  var _super = Playlists_createSuper(Players);

  function Players(props) {
    var _this;

    Playlists_classCallCheck(this, Players);

    _this = _super.call(this, props);
    _this.state = {
      loading: true,
      players: [],
      error: null
    };
    return _this;
  }

  Playlists_createClass(Players, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Playlists_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        var players;
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.prev = 0;
                _context.next = 3;
                return api.players();

              case 3:
                players = _context.sent;
                this.setState({
                  players: players,
                  loading: false
                });
                _context.next = 10;
                break;

              case 7:
                _context.prev = 7;
                _context.t0 = _context["catch"](0);
                this.setState({
                  error: _context.t0,
                  loading: false
                });

              case 10:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this, [[0, 7]]);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
  }, {
    key: "render",
    value: function render() {
      var content = null;

      if (!this.state.loading) {
        if (this.state.error !== null) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "danger",
            className: "oxi-center"
          }, this.state.error.toString());
        } else if (this.state.players.length === 0) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "warning",
            className: "oxi-center"
          }, "No active players!");
        } else {
          content = /*#__PURE__*/react.createElement(Table/* default */.Z, {
            className: "playlists",
            striped: true,
            bordered: true,
            hover: true
          }, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", null, "User"), /*#__PURE__*/react.createElement("th", {
            width: "1%"
          }, "Last\xA0Update"))), /*#__PURE__*/react.createElement("tbody", null, this.state.players.map(function (p) {
            var lastUpdate = "?";

            if (!!p.last_update) {
              lastUpdate = humanDurationSince(new Date(p.last_update));
            }

            return /*#__PURE__*/react.createElement("tr", {
              key: p.user_login
            }, /*#__PURE__*/react.createElement("td", null, /*#__PURE__*/react.createElement(react_router_dom/* Link */.rU, {
              alt: "Go to player",
              to: "/player/".concat(p.user_login)
            }, p.user_login)), /*#__PURE__*/react.createElement("td", {
              className: "playlists-last-update"
            }, lastUpdate));
          })));
        }
      }

      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement("h2", {
        className: "oxi-page-title"
      }, "Playlists"), /*#__PURE__*/react.createElement("p", {
        className: "oxi-center"
      }, "This page features people who have enabled remote playlists in OxidizeBot."), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), content);
    }
  }]);

  return Players;
}(react.Component);


;// CONCATENATED MODULE: ./src/assets/enable-remote-updates.png
/* harmony default export */ const enable_remote_updates = (__webpack_require__.p + "bbd7bc003812bdc208db92359644c0f0.png");
;// CONCATENATED MODULE: ./src/components/Player.js
function Player_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Player_typeof = function _typeof(obj) { return typeof obj; }; } else { Player_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Player_typeof(obj); }



















function Player_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Player_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Player_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Player_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Player_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Player_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Player_createClass(Constructor, protoProps, staticProps) { if (protoProps) Player_defineProperties(Constructor.prototype, protoProps); if (staticProps) Player_defineProperties(Constructor, staticProps); return Constructor; }

function Player_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Player_setPrototypeOf(subClass, superClass); }

function Player_setPrototypeOf(o, p) { Player_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Player_setPrototypeOf(o, p); }

function Player_createSuper(Derived) { var hasNativeReflectConstruct = Player_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Player_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Player_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Player_possibleConstructorReturn(this, result); }; }

function Player_possibleConstructorReturn(self, call) { if (call && (Player_typeof(call) === "object" || typeof call === "function")) { return call; } return Player_assertThisInitialized(self); }

function Player_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Player_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Player_getPrototypeOf(o) { Player_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Player_getPrototypeOf(o); }









var Player = /*#__PURE__*/function (_React$Component) {
  Player_inherits(Player, _React$Component);

  var _super = Player_createSuper(Player);

  function Player(props) {
    var _this;

    Player_classCallCheck(this, Player);

    _this = _super.call(this, props);
    _this.state = {
      error: null,
      loading: true,
      player: null
    };
    return _this;
  }

  Player_createClass(Player, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Player_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return this.refresh();

              case 2:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
  }, {
    key: "refresh",
    value: function () {
      var _refresh = Player_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var player;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                _context2.prev = 0;
                _context2.next = 3;
                return api.player(this.props.match.params.id);

              case 3:
                player = _context2.sent;
                this.setState({
                  error: null,
                  player: player,
                  loading: false
                });
                _context2.next = 10;
                break;

              case 7:
                _context2.prev = 7;
                _context2.t0 = _context2["catch"](0);
                this.setState({
                  error: _context2.t0,
                  player: null,
                  loading: false
                });

              case 10:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this, [[0, 7]]);
      }));

      function refresh() {
        return _refresh.apply(this, arguments);
      }

      return refresh;
    }()
    /**
     * Render the relevant error as an Alert.
     */

  }, {
    key: "renderError",
    value: function renderError(error) {
      if (error instanceof ApiError) {
        if (error.notFound()) {
          return /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "danger",
            className: "oxi-center"
          }, /*#__PURE__*/react.createElement("b", null, "Player not found."), /*#__PURE__*/react.createElement("div", {
            className: "player-not-found-hint"
          }, "Do you expect to see something here?", /*#__PURE__*/react.createElement("br", null), "Maybe you forgot to ", /*#__PURE__*/react.createElement("a", {
            href: "http://localhost:12345/settings?q=%5Eremote%2F"
          }, "enable remote updates"), " in your bot local settings:"), /*#__PURE__*/react.createElement("div", {
            className: "player-not-found-hint-image"
          }, /*#__PURE__*/react.createElement("img", {
            src: enable_remote_updates_namespaceObject
          })));
        }
      }

      return /*#__PURE__*/react.createElement(Alert/* default */.Z, {
        variant: "danger",
        className: "oxi-center"
      }, error.toString());
    }
  }, {
    key: "render",
    value: function render() {
      var content = null;

      if (!this.state.loading) {
        if (this.state.error !== null) {
          content = this.renderError(this.state.error);
        } else if (this.state.player === null) {
          content = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
            variant: "warning",
            className: "oxi-center"
          }, "User doesn't have an active player!");
        } else {
          content = /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement(Table/* default */.Z, {
            className: "player",
            striped: true,
            bordered: true,
            hover: true
          }, /*#__PURE__*/react.createElement("thead", null, /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("th", null), /*#__PURE__*/react.createElement("th", {
            scope: "col"
          }, "Song"), /*#__PURE__*/react.createElement("th", {
            scope: "col"
          }, "Artist"), /*#__PURE__*/react.createElement("th", {
            scope: "col"
          }, "Length"), /*#__PURE__*/react.createElement("th", {
            scope: "col"
          }, "Requested By"))), /*#__PURE__*/react.createElement("tbody", null, this.state.player.items.map(function (_ref, index) {
            var name = _ref.name,
                track_url = _ref.track_url,
                artists = _ref.artists,
                duration = _ref.duration,
                user = _ref.user;
            var classes = "";
            var current = index;

            if (index == 0) {
              current = /*#__PURE__*/react.createElement("span", {
                title: "Current Song"
              }, "\u25B6");
              classes = "oxi-current";
            }

            var userInfo = null;

            if (user !== null) {
              userInfo = /*#__PURE__*/react.createElement("a", {
                href: "https://twitch.tv/".concat(user)
              }, user);
            } else {
              userInfo = /*#__PURE__*/react.createElement("a", {
                href: "https://awoiaf.westeros.org/index.php/Faceless_Men"
              }, /*#__PURE__*/react.createElement("em", null, "No One"));
            }

            return /*#__PURE__*/react.createElement("tr", {
              key: index,
              className: classes
            }, /*#__PURE__*/react.createElement("th", null, current), /*#__PURE__*/react.createElement("td", null, /*#__PURE__*/react.createElement("a", {
              href: track_url
            }, name)), /*#__PURE__*/react.createElement("td", null, artists), /*#__PURE__*/react.createElement("td", null, duration), /*#__PURE__*/react.createElement("td", null, userInfo));
          }))));
        }
      }

      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement("h2", {
        className: "oxi-page-title"
      }, "Playlist for ", this.props.match.params.id), /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), content);
    }
  }]);

  return Player;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/core-js/modules/web.timers.js
var web_timers = __webpack_require__(2564);
// EXTERNAL MODULE: ./node_modules/core-js/modules/web.url.js
var web_url = __webpack_require__(285);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.string.search.js
var es_string_search = __webpack_require__(4765);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.array.find.js
var es_array_find = __webpack_require__(9826);
// EXTERNAL MODULE: ./node_modules/core-js/modules/es.object.assign.js
var es_object_assign = __webpack_require__(9601);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/InputGroup.js
var InputGroup = __webpack_require__(2318);
// EXTERNAL MODULE: ./node_modules/react-bootstrap/esm/FormControl.js
var FormControl = __webpack_require__(4716);
;// CONCATENATED MODULE: ./src/components/UserPrompt.js
function UserPrompt_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { UserPrompt_typeof = function _typeof(obj) { return typeof obj; }; } else { UserPrompt_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return UserPrompt_typeof(obj); }














function UserPrompt_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function UserPrompt_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { UserPrompt_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { UserPrompt_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function UserPrompt_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function UserPrompt_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function UserPrompt_createClass(Constructor, protoProps, staticProps) { if (protoProps) UserPrompt_defineProperties(Constructor.prototype, protoProps); if (staticProps) UserPrompt_defineProperties(Constructor, staticProps); return Constructor; }

function UserPrompt_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) UserPrompt_setPrototypeOf(subClass, superClass); }

function UserPrompt_setPrototypeOf(o, p) { UserPrompt_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return UserPrompt_setPrototypeOf(o, p); }

function UserPrompt_createSuper(Derived) { var hasNativeReflectConstruct = UserPrompt_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = UserPrompt_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = UserPrompt_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return UserPrompt_possibleConstructorReturn(this, result); }; }

function UserPrompt_possibleConstructorReturn(self, call) { if (call && (UserPrompt_typeof(call) === "object" || typeof call === "function")) { return call; } return UserPrompt_assertThisInitialized(self); }

function UserPrompt_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function UserPrompt_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function UserPrompt_getPrototypeOf(o) { UserPrompt_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return UserPrompt_getPrototypeOf(o); }






var UserPrompt = /*#__PURE__*/function (_React$Component) {
  UserPrompt_inherits(UserPrompt, _React$Component);

  var _super = UserPrompt_createSuper(UserPrompt);

  function UserPrompt(props) {
    UserPrompt_classCallCheck(this, UserPrompt);

    return _super.call(this, props);
  }

  UserPrompt_createClass(UserPrompt, [{
    key: "login",
    value: function () {
      var _login = UserPrompt_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        var result;
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.next = 2;
                return api.authLogin();

              case 2:
                result = _context.sent;
                location.href = result.auth_url;

              case 4:
              case "end":
                return _context.stop();
            }
          }
        }, _callee);
      }));

      function login() {
        return _login.apply(this, arguments);
      }

      return login;
    }()
  }, {
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement(Alert/* default */.Z, {
        variant: "warning",
        className: "oxi-center"
      }, /*#__PURE__*/react.createElement("div", {
        className: "mb-3"
      }, "This page requires you to sign in!"), /*#__PURE__*/react.createElement(Form/* default */.Z, null, /*#__PURE__*/react.createElement(Button/* default */.Z, {
        size: "xl",
        onClick: this.login.bind(this),
        title: "Sign in through Twitch"
      }, "Sign in with ", /*#__PURE__*/react.createElement("img", {
        src: twitch,
        height: "16px",
        width: "48px",
        alt: "twitch"
      })))));
    }
  }]);

  return UserPrompt;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/copy-to-clipboard/index.js
var copy_to_clipboard = __webpack_require__(640);
var copy_to_clipboard_default = /*#__PURE__*/__webpack_require__.n(copy_to_clipboard);
;// CONCATENATED MODULE: ./src/components/Connection.js
function Connection_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Connection_typeof = function _typeof(obj) { return typeof obj; }; } else { Connection_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Connection_typeof(obj); }















function Connection_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Connection_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Connection_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Connection_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Connection_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Connection_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Connection_createClass(Constructor, protoProps, staticProps) { if (protoProps) Connection_defineProperties(Constructor.prototype, protoProps); if (staticProps) Connection_defineProperties(Constructor, staticProps); return Constructor; }

function Connection_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Connection_setPrototypeOf(subClass, superClass); }

function Connection_setPrototypeOf(o, p) { Connection_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Connection_setPrototypeOf(o, p); }

function Connection_createSuper(Derived) { var hasNativeReflectConstruct = Connection_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Connection_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Connection_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Connection_possibleConstructorReturn(this, result); }; }

function Connection_possibleConstructorReturn(self, call) { if (call && (Connection_typeof(call) === "object" || typeof call === "function")) { return call; } return Connection_assertThisInitialized(self); }

function Connection_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Connection_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Connection_getPrototypeOf(o) { Connection_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Connection_getPrototypeOf(o); }







var Connection = /*#__PURE__*/function (_React$Component) {
  Connection_inherits(Connection, _React$Component);

  var _super = Connection_createSuper(Connection);

  function Connection(props) {
    var _this;

    Connection_classCallCheck(this, Connection);

    _this = _super.call(this, props);
    _this.state = {
      copied: false
    };
    _this.clearCopied = null;
    return _this;
  }

  Connection_createClass(Connection, [{
    key: "connect",
    value: function () {
      var _connect = Connection_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        var result;
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                _context.prev = 0;
                _context.next = 3;
                return api.connectionsCreate(this.props.id);

              case 3:
                result = _context.sent;
                location.href = result.auth_url;
                _context.next = 10;
                break;

              case 7:
                _context.prev = 7;
                _context.t0 = _context["catch"](0);
                this.props.onError(_context.t0);

              case 10:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this, [[0, 7]]);
      }));

      function connect() {
        return _connect.apply(this, arguments);
      }

      return connect;
    }()
  }, {
    key: "copy",
    value: function () {
      var _copy2 = Connection_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var _this2 = this;

        var result;
        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                if (this.clearCopied !== null) {
                  clearTimeout(this.clearCopied);
                  this.clearCopied = null;
                }

                _context2.next = 3;
                return api.connectionsCreate(this.props.id);

              case 3:
                result = _context2.sent;

                copy_to_clipboard_default()(result.auth_url);

                this.setState({
                  copied: true
                });
                this.clearCopied = setTimeout(function () {
                  return _this2.setState({
                    copied: false
                  });
                }, 2000);

              case 7:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this);
      }));

      function copy() {
        return _copy2.apply(this, arguments);
      }

      return copy;
    }()
  }, {
    key: "disconnect",
    value: function () {
      var _disconnect = Connection_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3() {
        var result;
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                _context3.next = 2;
                return api.connectionsRemove(this.props.id);

              case 2:
                result = _context3.sent;

                if (this.props.onDisconnect) {
                  this.props.onDisconnect();
                }

              case 4:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this);
      }));

      function disconnect() {
        return _disconnect.apply(this, arguments);
      }

      return disconnect;
    }()
  }, {
    key: "icon",
    value: function icon() {
      switch (this.props.type) {
        case "twitch":
          return /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: ['fab', 'twitch']
          });

        case "youtube":
          return /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: ['fab', 'youtube']
          });

        case "spotify":
          return /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: ['fab', 'spotify']
          });

        default:
          return /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "globe"
          });
      }
    }
  }, {
    key: "meta",
    value: function meta() {
      var account = null;

      switch (this.props.type) {
        case "twitch":
          if (!this.props.meta || !this.props.meta.login) {
            return null;
          }

          account = /*#__PURE__*/react.createElement("a", {
            href: "https://twitch.tv/".concat(this.props.meta.login)
          }, /*#__PURE__*/react.createElement("b", null, this.props.meta.login));
          break;

        case "spotify":
          if (!this.props.meta || !this.props.meta.display_name) {
            return null;
          }

          var product = null;

          if (this.props.meta.product) {
            product = /*#__PURE__*/react.createElement(react.Fragment, null, " (", this.props.meta.product, ")");
          }

          account = /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("b", null, this.props.meta.display_name), product);

          if (this.props.meta.external_urls && this.props.meta.external_urls.spotify) {
            account = /*#__PURE__*/react.createElement("a", {
              href: this.props.meta.external_urls.spotify
            }, account);
          }

          break;

        default:
          return null;
      }

      return /*#__PURE__*/react.createElement("div", {
        className: "oxi-connected-meta"
      }, "Connected account: ", account);
    }
  }, {
    key: "validate",
    value: function validate() {
      if (this.props.outdated) {
        return /*#__PURE__*/react.createElement("div", {
          className: "oxi-connected-validate danger"
        }, /*#__PURE__*/react.createElement("b", null, "Connection is outdated, in order to use it properly it needs to be refreshed!"));
      }

      switch (this.props.type) {
        case "spotify":
          if (!this.props.meta || !this.props.meta.product) {
            return null;
          }

          if (this.props.meta.product === "premium") {
            return null;
          }

          return /*#__PURE__*/react.createElement("div", {
            className: "oxi-connected-validate danger"
          }, /*#__PURE__*/react.createElement("b", null, "You need a Premium Spotify Account"));

        default:
          return null;
      }
    }
  }, {
    key: "render",
    value: function render() {
      var _this3 = this;

      var icon = this.icon();
      var buttons = [];
      var button = null;

      if (this.props.connected !== null) {
        var copy = false;

        if (!this.props.connected) {
          buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
            key: "connect",
            disabled: currentUser === null,
            size: "sm",
            variant: "primary",
            onClick: function onClick() {
              return _this3.connect();
            },
            title: "Connect"
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "plug"
          })));
          copy = true;
        }

        if (this.props.outdated) {
          buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
            key: "refresh",
            size: "sm",
            variant: "warning",
            onClick: function onClick() {
              return _this3.connect();
            },
            title: "Connection is outdated and need to be refreshed!"
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "sync"
          })));
          copy = true;
        }

        if (copy) {
          if (this.state.copied) {
            buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
              key: "copy",
              size: "sm",
              variant: "success",
              disabled: true
            }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
              icon: "check"
            })));
          } else {
            buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
              key: "copy",
              disabled: currentUser === null,
              size: "sm",
              variant: "success",
              onClick: function onClick() {
                return _this3.copy();
              },
              title: "Copy connection URL to clipboard"
            }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
              icon: "copy"
            })));
          }
        }

        if (this.props.connected) {
          buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
            key: "remove",
            size: "sm",
            variant: "danger",
            onClick: function onClick() {
              return _this3.disconnect();
            },
            title: "Remove connection"
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "trash"
          })));
        } else {
          buttons.push( /*#__PURE__*/react.createElement(Button/* default */.Z, {
            disabled: true,
            key: "remove",
            size: "sm",
            variant: "light",
            title: "Connection not present"
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "trash"
          })));
        }
      }

      buttons = /*#__PURE__*/react.createElement(ButtonGroup/* default */.Z, null, buttons);
      var meta = this.meta();
      var validate = this.validate();
      return /*#__PURE__*/react.createElement("tr", null, /*#__PURE__*/react.createElement("td", {
        className: "oxi-connected"
      }, /*#__PURE__*/react.createElement("div", {
        className: "oxi-connected-title"
      }, icon, " ", this.props.title), meta, validate, /*#__PURE__*/react.createElement("div", {
        className: "oxi-connected-description"
      }, this.props.description)), /*#__PURE__*/react.createElement("td", {
        align: "right"
      }, buttons));
    }
  }]);

  return Connection;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/Connections.js
function Connections_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Connections_typeof = function _typeof(obj) { return typeof obj; }; } else { Connections_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Connections_typeof(obj); }

function _extends() { _extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return _extends.apply(this, arguments); }

function Connections_slicedToArray(arr, i) { return Connections_arrayWithHoles(arr) || Connections_iterableToArrayLimit(arr, i) || Connections_unsupportedIterableToArray(arr, i) || Connections_nonIterableRest(); }

function Connections_nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }

function Connections_iterableToArrayLimit(arr, i) { if (typeof Symbol === "undefined" || !(Symbol.iterator in Object(arr))) return; var _arr = []; var _n = true; var _d = false; var _e = undefined; try { for (var _i = arr[Symbol.iterator](), _s; !(_n = (_s = _i.next()).done); _n = true) { _arr.push(_s.value); if (i && _arr.length === i) break; } } catch (err) { _d = true; _e = err; } finally { try { if (!_n && _i["return"] != null) _i["return"](); } finally { if (_d) throw _e; } } return _arr; }

function Connections_arrayWithHoles(arr) { if (Array.isArray(arr)) return arr; }



function Connections_asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { Promise.resolve(value).then(_next, _throw); } }

function Connections_asyncToGenerator(fn) { return function () { var self = this, args = arguments; return new Promise(function (resolve, reject) { var gen = fn.apply(self, args); function _next(value) { Connections_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value); } function _throw(err) { Connections_asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err); } _next(undefined); }); }; }

function Connections_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Connections_setPrototypeOf(subClass, superClass); }

function Connections_setPrototypeOf(o, p) { Connections_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Connections_setPrototypeOf(o, p); }

function Connections_createSuper(Derived) { var hasNativeReflectConstruct = Connections_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Connections_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Connections_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Connections_possibleConstructorReturn(this, result); }; }

function Connections_possibleConstructorReturn(self, call) { if (call && (Connections_typeof(call) === "object" || typeof call === "function")) { return call; } return Connections_assertThisInitialized(self); }

function Connections_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Connections_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Connections_getPrototypeOf(o) { Connections_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Connections_getPrototypeOf(o); }

function Connections_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Connections_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e2) { throw _e2; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e3) { didErr = true; err = _e3; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Connections_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Connections_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Connections_arrayLikeToArray(o, minLen); }

function Connections_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }
























function Connections_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Connections_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Connections_createClass(Constructor, protoProps, staticProps) { if (protoProps) Connections_defineProperties(Constructor.prototype, protoProps); if (staticProps) Connections_defineProperties(Constructor, staticProps); return Constructor; }










var CountDown = /*#__PURE__*/function () {
  function CountDown(count, call, end) {
    Connections_classCallCheck(this, CountDown);

    this.count = count;
    var self = this;
    this.interval = setInterval(function () {
      if (self.count <= 1) {
        self.stop();
        end();
      } else {
        self.count -= 1;
        call(self.count);
      }
    }, 1000);
    call(self.count);
  }

  Connections_createClass(CountDown, [{
    key: "stop",
    value: function stop() {
      if (this.interval !== null) {
        clearInterval(this.interval);
        this.interval = null;
      }
    }
  }]);

  return CountDown;
}();

function baseConnections() {
  var connections = {};

  var _iterator = Connections_createForOfIteratorHelper(currentConnections),
      _step;

  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var c = _step.value;
      connections[c.id] = null;
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }

  return connections;
}

var Connections = /*#__PURE__*/function (_React$Component) {
  Connections_inherits(Connections, _React$Component);

  var _super = Connections_createSuper(Connections);

  function Connections(props) {
    var _this;

    Connections_classCallCheck(this, Connections);

    _this = _super.call(this, props);
    var q = new URLSearchParams(props.location.search);
    _this.state = {
      loading: true,
      error: null,
      connections: baseConnections(),
      key: null,
      showKeyCount: null,
      justConnected: q.get("connected")
    };
    _this.showKey = null;
    return _this;
  }

  Connections_createClass(Connections, [{
    key: "componentDidMount",
    value: function () {
      var _componentDidMount = Connections_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee() {
        return regeneratorRuntime.wrap(function _callee$(_context) {
          while (1) {
            switch (_context.prev = _context.next) {
              case 0:
                if (!(currentUser !== null)) {
                  _context.next = 9;
                  break;
                }

                _context.prev = 1;
                _context.next = 4;
                return this.refreshConnections();

              case 4:
                _context.next = 9;
                break;

              case 6:
                _context.prev = 6;
                _context.t0 = _context["catch"](1);
                this.setState({
                  error: _context.t0
                });

              case 9:
                this.setState({
                  loading: false
                });

              case 10:
              case "end":
                return _context.stop();
            }
          }
        }, _callee, this, [[1, 6]]);
      }));

      function componentDidMount() {
        return _componentDidMount.apply(this, arguments);
      }

      return componentDidMount;
    }()
  }, {
    key: "refreshConnections",
    value: function () {
      var _refreshConnections = Connections_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee2() {
        var _yield$Promise$all, _yield$Promise$all2, update, key, connections, _iterator2, _step2, c, _iterator3, _step3, u;

        return regeneratorRuntime.wrap(function _callee2$(_context2) {
          while (1) {
            switch (_context2.prev = _context2.next) {
              case 0:
                _context2.next = 2;
                return Promise.all([api.connectionsList(), api.getKey()]);

              case 2:
                _yield$Promise$all = _context2.sent;
                _yield$Promise$all2 = Connections_slicedToArray(_yield$Promise$all, 2);
                update = _yield$Promise$all2[0];
                key = _yield$Promise$all2[1];
                connections = {};
                _iterator2 = Connections_createForOfIteratorHelper(currentConnections);

                try {
                  for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
                    c = _step2.value;
                    connections[c.id] = {
                      connected: false
                    };
                  }
                } catch (err) {
                  _iterator2.e(err);
                } finally {
                  _iterator2.f();
                }

                _iterator3 = Connections_createForOfIteratorHelper(update);

                try {
                  for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
                    u = _step3.value;
                    connections[u.id] = {
                      outdated: u.outdated,
                      meta: u.meta,
                      connected: true
                    };
                  }
                } catch (err) {
                  _iterator3.e(err);
                } finally {
                  _iterator3.f();
                }

                this.setState({
                  connections: connections,
                  key: key.key
                });

              case 12:
              case "end":
                return _context2.stop();
            }
          }
        }, _callee2, this);
      }));

      function refreshConnections() {
        return _refreshConnections.apply(this, arguments);
      }

      return refreshConnections;
    }()
  }, {
    key: "onDisconnect",
    value: function () {
      var _onDisconnect = Connections_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee3() {
        return regeneratorRuntime.wrap(function _callee3$(_context3) {
          while (1) {
            switch (_context3.prev = _context3.next) {
              case 0:
                this.setState({
                  error: null
                });
                _context3.prev = 1;
                _context3.next = 4;
                return this.refreshConnections();

              case 4:
                _context3.next = 9;
                break;

              case 6:
                _context3.prev = 6;
                _context3.t0 = _context3["catch"](1);
                this.onError(_context3.t0);

              case 9:
              case "end":
                return _context3.stop();
            }
          }
        }, _callee3, this, [[1, 6]]);
      }));

      function onDisconnect() {
        return _onDisconnect.apply(this, arguments);
      }

      return onDisconnect;
    }()
  }, {
    key: "generateKey",
    value: function () {
      var _generateKey = Connections_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee4() {
        var key;
        return regeneratorRuntime.wrap(function _callee4$(_context4) {
          while (1) {
            switch (_context4.prev = _context4.next) {
              case 0:
                this.setState({
                  error: null
                });
                _context4.prev = 1;
                _context4.next = 4;
                return api.createKey();

              case 4:
                key = _context4.sent;
                this.setState({
                  key: key.key
                });
                _context4.next = 12;
                break;

              case 8:
                _context4.prev = 8;
                _context4.t0 = _context4["catch"](1);
                this.setState({
                  error: _context4.t0
                });
                return _context4.abrupt("return");

              case 12:
              case "end":
                return _context4.stop();
            }
          }
        }, _callee4, this, [[1, 8]]);
      }));

      function generateKey() {
        return _generateKey.apply(this, arguments);
      }

      return generateKey;
    }()
  }, {
    key: "clearKey",
    value: function () {
      var _clearKey = Connections_asyncToGenerator( /*#__PURE__*/regeneratorRuntime.mark(function _callee5() {
        return regeneratorRuntime.wrap(function _callee5$(_context5) {
          while (1) {
            switch (_context5.prev = _context5.next) {
              case 0:
                this.setState({
                  error: null
                });
                _context5.prev = 1;
                _context5.next = 4;
                return api.deleteKey();

              case 4:
                this.setState({
                  key: null
                });
                this.hideKey();
                _context5.next = 12;
                break;

              case 8:
                _context5.prev = 8;
                _context5.t0 = _context5["catch"](1);
                this.setState({
                  error: _context5.t0
                });
                return _context5.abrupt("return");

              case 12:
              case "end":
                return _context5.stop();
            }
          }
        }, _callee5, this, [[1, 8]]);
      }));

      function clearKey() {
        return _clearKey.apply(this, arguments);
      }

      return clearKey;
    }()
  }, {
    key: "onError",
    value: function onError(e) {
      this.setState({
        error: e
      });
    }
  }, {
    key: "send",
    value: function send() {
      var query = "";

      if (this.state.key) {
        query = "?key=".concat(encodeURIComponent(this.state.key));
      }

      location.href = "http://localhost:12345/api/auth/key".concat(query);
    }
  }, {
    key: "hideKey",
    value: function hideKey() {
      if (this.showKey !== null) {
        this.showKey.stop();
        this.showKey = null;
      }

      this.setState({
        showKeyCount: null
      });
    }
  }, {
    key: "showKeyFor",
    value: function showKeyFor(count) {
      var _this2 = this;

      if (this.showKey !== null) {
        this.showKey.stop();
        this.showKey = null;
      }

      this.showKey = new CountDown(count, function (i) {
        _this2.setState({
          showKeyCount: i
        });
      }, function () {
        _this2.setState({
          showKeyCount: null
        });
      });
    }
  }, {
    key: "renderJustConnected",
    value: function renderJustConnected() {
      var _this3 = this;

      if (!this.state.justConnected) {
        return null;
      }

      var connected = currentConnections.find(function (c) {
        return c.id === _this3.state.justConnected;
      });

      if (connected === null) {
        return null;
      }

      var otherAccount = null;

      if (currentUser === null) {
        otherAccount = /*#__PURE__*/react.createElement(react.Fragment, null, " (Another Account)");
      }

      return /*#__PURE__*/react.createElement(Alert/* default */.Z, {
        variant: "info",
        className: "oxi-center"
      }, /*#__PURE__*/react.createElement("b", null, "Successfully connected ", connected.title), otherAccount);
    }
  }, {
    key: "render",
    value: function render() {
      var _this4 = this;

      var justConnected = this.renderJustConnected();
      var error = null;

      if (this.state.error !== null) {
        error = /*#__PURE__*/react.createElement(Alert/* default */.Z, {
          variant: "danger"
        }, this.state.error.toString());
      }

      var showKey = null;

      if (this.state.key !== null) {
        if (this.state.showKeyCount !== null) {
          showKey = /*#__PURE__*/react.createElement(Button/* default */.Z, {
            variant: "light",
            onClick: function onClick() {
              return _this4.hideKey();
            }
          }, this.state.showKeyCount, " ", /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "eye-slash",
            title: "Hide key"
          }));
        } else {
          showKey = /*#__PURE__*/react.createElement(Button/* default */.Z, {
            variant: "light",
            onClick: function onClick() {
              return _this4.showKeyFor(10);
            },
            title: "Click to show secret key for 10 seconds"
          }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
            icon: "eye"
          }));
        }
      }

      var value = "";
      var placeholder = null;
      var clear = null;
      var generate = null;
      var send;

      if (this.state.showKeyCount !== null && this.state.key != null) {
        value = this.state.key;
      }

      if (this.state.key === null) {
        placeholder = "no key available";
        generate = /*#__PURE__*/react.createElement(Button/* default */.Z, {
          disabled: currentUser === null,
          variant: "primary",
          onClick: function onClick() {
            return _this4.generateKey();
          },
          title: "Generate a new secret key."
        }, "Generate");
      } else {
        placeholder = "key hidden";
        clear = /*#__PURE__*/react.createElement(Button/* default */.Z, {
          variant: "danger",
          disabled: this.state.key === null,
          onClick: function onClick() {
            return _this4.clearKey();
          },
          title: "Clear the current key without regenerating it."
        }, "Clear");
        generate = /*#__PURE__*/react.createElement(Button/* default */.Z, {
          variant: "primary",
          onClick: function onClick() {
            return _this4.generateKey();
          },
          title: "Create a new key, invalidating the existing key."
        }, "Regenerate");
        send = /*#__PURE__*/react.createElement(Button/* default */.Z, {
          variant: "info",
          title: "Send key to bot",
          onClick: function onClick() {
            return _this4.send();
          }
        }, /*#__PURE__*/react.createElement(index_es/* FontAwesomeIcon */.G, {
          icon: "share"
        }));
      }

      var key = /*#__PURE__*/react.createElement(Form/* default */.Z, {
        className: "mb-3"
      }, /*#__PURE__*/react.createElement(InputGroup/* default */.Z, null, /*#__PURE__*/react.createElement(FormControl/* default */.Z, {
        readOnly: true,
        value: value,
        placeholder: placeholder
      }), /*#__PURE__*/react.createElement(InputGroup/* default.Append */.Z.Append, null, showKey, clear, generate, send)));
      var userPrompt = null;

      if (justConnected === null && currentUser === null) {
        userPrompt = /*#__PURE__*/react.createElement(UserPrompt, null);
      }

      var content = null;

      if (!this.state.loading) {
        content = /*#__PURE__*/react.createElement(react.Fragment, null, /*#__PURE__*/react.createElement("p", null, "Connections allow OxidizeBot to access third party services like Spotify and Twitch. This might be necessary for the bot to provide certain features, like viewer-driven song requests."), /*#__PURE__*/react.createElement("h4", null, "Secret Key"), /*#__PURE__*/react.createElement("p", null, "This key should be configured in your bot to allow it to communicate with this service."), key, /*#__PURE__*/react.createElement("h4", null, "Connections"), /*#__PURE__*/react.createElement("p", null, "Each connection adds capabilities to OxidizeBot. You'll have to enable and authenticate them here."), /*#__PURE__*/react.createElement(Table/* default */.Z, null, /*#__PURE__*/react.createElement("tbody", null, currentConnections.map(function (c, index) {
          return /*#__PURE__*/react.createElement(Connection, _extends({
            key: index,
            onDisconnect: function onDisconnect() {
              return _this4.onDisconnect(c.id);
            },
            onError: function onError(e) {
              return _this4.onError(e);
            }
          }, c, _this4.state.connections[c.id]));
        }))));
      }

      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement("h2", {
        className: "oxi-page-title"
      }, "My Connections"), justConnected, userPrompt, /*#__PURE__*/react.createElement(Loading, {
        isLoading: this.state.loading
      }), error, content);
    }
  }]);

  return Connections;
}(react.Component);


// EXTERNAL MODULE: ./loaders/toml-loader/index.js!../shared/commands.toml
var commands = __webpack_require__(1484);
var commands_default = /*#__PURE__*/__webpack_require__.n(commands);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.set.js
var es_set = __webpack_require__(6810);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.object.to-string.js
var modules_es_object_to_string = __webpack_require__(5283);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.string.iterator.js
var modules_es_string_iterator = __webpack_require__(7521);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.array.iterator.js
var modules_es_array_iterator = __webpack_require__(6437);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/web.dom-collections.iterator.js
var modules_web_dom_collections_iterator = __webpack_require__(6663);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.string.split.js
var es_string_split = __webpack_require__(1916);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.regexp.exec.js
var modules_es_regexp_exec = __webpack_require__(8348);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.string.replace.js
var es_string_replace = __webpack_require__(2339);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.string.starts-with.js
var modules_es_string_starts_with = __webpack_require__(6289);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/web.url.js
var modules_web_url = __webpack_require__(9818);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.string.search.js
var modules_es_string_search = __webpack_require__(3324);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.array.filter.js
var modules_es_array_filter = __webpack_require__(2135);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.array.map.js
var modules_es_array_map = __webpack_require__(7942);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.function.name.js
var modules_es_function_name = __webpack_require__(9550);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.object.assign.js
var modules_es_object_assign = __webpack_require__(707);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.array.concat.js
var modules_es_array_concat = __webpack_require__(467);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.array.slice.js
var modules_es_array_slice = __webpack_require__(1778);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.array.from.js
var modules_es_array_from = __webpack_require__(6804);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.symbol.js
var modules_es_symbol = __webpack_require__(2017);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.symbol.description.js
var modules_es_symbol_description = __webpack_require__(7658);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.symbol.iterator.js
var modules_es_symbol_iterator = __webpack_require__(3239);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.object.set-prototype-of.js
var modules_es_object_set_prototype_of = __webpack_require__(5982);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.object.get-prototype-of.js
var modules_es_object_get_prototype_of = __webpack_require__(3488);
// EXTERNAL MODULE: ../shared-ui/node_modules/core-js/modules/es.reflect.construct.js
var modules_es_reflect_construct = __webpack_require__(3013);
// EXTERNAL MODULE: ../shared-ui/node_modules/react-markdown/lib/react-markdown.js
var react_markdown = __webpack_require__(9209);
var react_markdown_default = /*#__PURE__*/__webpack_require__.n(react_markdown);
;// CONCATENATED MODULE: ../shared-ui/utils.js


function Header(props) {
  return /*#__PURE__*/node_modules_react.createElement((react_markdown_default()), {
    source: props.source
  });
}
function Content(props) {
  return /*#__PURE__*/node_modules_react.createElement((react_markdown_default()), {
    source: props.source
  });
}
function ExampleContent(props) {
  return /*#__PURE__*/node_modules_react.createElement("pre", null, /*#__PURE__*/node_modules_react.createElement("code", null, props.source));
}
;// CONCATENATED MODULE: ../shared-ui/components/Example.js
function Example_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Example_typeof = function _typeof(obj) { return typeof obj; }; } else { Example_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Example_typeof(obj); }













function Example_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Example_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Example_createClass(Constructor, protoProps, staticProps) { if (protoProps) Example_defineProperties(Constructor.prototype, protoProps); if (staticProps) Example_defineProperties(Constructor, staticProps); return Constructor; }

function Example_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Example_setPrototypeOf(subClass, superClass); }

function Example_setPrototypeOf(o, p) { Example_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Example_setPrototypeOf(o, p); }

function Example_createSuper(Derived) { var hasNativeReflectConstruct = Example_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Example_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Example_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Example_possibleConstructorReturn(this, result); }; }

function Example_possibleConstructorReturn(self, call) { if (call && (Example_typeof(call) === "object" || typeof call === "function")) { return call; } return Example_assertThisInitialized(self); }

function Example_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Example_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Example_getPrototypeOf(o) { Example_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Example_getPrototypeOf(o); }




var Example = /*#__PURE__*/function (_React$Component) {
  Example_inherits(Example, _React$Component);

  var _super = Example_createSuper(Example);

  function Example(props) {
    Example_classCallCheck(this, Example);

    return _super.call(this, props);
  }

  Example_createClass(Example, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/node_modules_react.createElement(node_modules_react.Fragment, null, /*#__PURE__*/node_modules_react.createElement("div", {
        className: "oxi-example-name"
      }, /*#__PURE__*/node_modules_react.createElement("b", null, "Example:"), " ", /*#__PURE__*/node_modules_react.createElement(Header, {
        source: this.props.name
      })), /*#__PURE__*/node_modules_react.createElement("div", {
        className: "oxi-example-content"
      }, /*#__PURE__*/node_modules_react.createElement(ExampleContent, {
        source: this.props.content
      })));
    }
  }]);

  return Example;
}(node_modules_react.Component);


;// CONCATENATED MODULE: ../shared-ui/components/Command.js
function Command_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Command_typeof = function _typeof(obj) { return typeof obj; }; } else { Command_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Command_typeof(obj); }

function Command_extends() { Command_extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return Command_extends.apply(this, arguments); }















function Command_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Command_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Command_createClass(Constructor, protoProps, staticProps) { if (protoProps) Command_defineProperties(Constructor.prototype, protoProps); if (staticProps) Command_defineProperties(Constructor, staticProps); return Constructor; }

function Command_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Command_setPrototypeOf(subClass, superClass); }

function Command_setPrototypeOf(o, p) { Command_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Command_setPrototypeOf(o, p); }

function Command_createSuper(Derived) { var hasNativeReflectConstruct = Command_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Command_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Command_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Command_possibleConstructorReturn(this, result); }; }

function Command_possibleConstructorReturn(self, call) { if (call && (Command_typeof(call) === "object" || typeof call === "function")) { return call; } return Command_assertThisInitialized(self); }

function Command_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Command_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Command_getPrototypeOf(o) { Command_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Command_getPrototypeOf(o); }





var Command = /*#__PURE__*/function (_React$Component) {
  Command_inherits(Command, _React$Component);

  var _super = Command_createSuper(Command);

  function Command(props) {
    Command_classCallCheck(this, Command);

    return _super.call(this, props);
  }

  Command_createClass(Command, [{
    key: "render",
    value: function render() {
      var examples = null;

      if (this.props.examples && this.props.examples.length > 0) {
        examples = (this.props.examples || []).map(function (e, i) {
          return /*#__PURE__*/node_modules_react.createElement(Example, Command_extends({
            key: i
          }, e));
        });
      }

      return /*#__PURE__*/node_modules_react.createElement(node_modules_react.Fragment, null, /*#__PURE__*/node_modules_react.createElement("tr", null, /*#__PURE__*/node_modules_react.createElement("td", {
        className: "oxi-command"
      }, /*#__PURE__*/node_modules_react.createElement("div", {
        className: "oxi-command-name"
      }, /*#__PURE__*/node_modules_react.createElement(Header, {
        source: this.props.name
      })), /*#__PURE__*/node_modules_react.createElement(Content, {
        source: this.props.content
      }), examples)));
    }
  }]);

  return Command;
}(node_modules_react.Component);


;// CONCATENATED MODULE: ../shared-ui/components/CommandGroup.js
function CommandGroup_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { CommandGroup_typeof = function _typeof(obj) { return typeof obj; }; } else { CommandGroup_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return CommandGroup_typeof(obj); }

function CommandGroup_extends() { CommandGroup_extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return CommandGroup_extends.apply(this, arguments); }















function CommandGroup_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function CommandGroup_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function CommandGroup_createClass(Constructor, protoProps, staticProps) { if (protoProps) CommandGroup_defineProperties(Constructor.prototype, protoProps); if (staticProps) CommandGroup_defineProperties(Constructor, staticProps); return Constructor; }

function CommandGroup_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) CommandGroup_setPrototypeOf(subClass, superClass); }

function CommandGroup_setPrototypeOf(o, p) { CommandGroup_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return CommandGroup_setPrototypeOf(o, p); }

function CommandGroup_createSuper(Derived) { var hasNativeReflectConstruct = CommandGroup_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = CommandGroup_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = CommandGroup_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return CommandGroup_possibleConstructorReturn(this, result); }; }

function CommandGroup_possibleConstructorReturn(self, call) { if (call && (CommandGroup_typeof(call) === "object" || typeof call === "function")) { return call; } return CommandGroup_assertThisInitialized(self); }

function CommandGroup_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function CommandGroup_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function CommandGroup_getPrototypeOf(o) { CommandGroup_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return CommandGroup_getPrototypeOf(o); }





var CommandGroup = /*#__PURE__*/function (_React$Component) {
  CommandGroup_inherits(CommandGroup, _React$Component);

  var _super = CommandGroup_createSuper(CommandGroup);

  function CommandGroup(props) {
    var _this;

    CommandGroup_classCallCheck(this, CommandGroup);

    _this = _super.call(this, props);
    _this.state = {
      expanded: false
    };
    return _this;
  }

  CommandGroup_createClass(CommandGroup, [{
    key: "toggle",
    value: function toggle(expanded) {
      this.setState({
        expanded: expanded
      });
    }
  }, {
    key: "render",
    value: function render() {
      var _this2 = this;

      var commands = null;
      var expand = this.state.expanded || !this.props.expandable || !!this.props.modified;

      if (this.props.commands && this.props.commands.length > 0 && expand) {
        commands = /*#__PURE__*/node_modules_react.createElement("table", {
          className: "table table-dark table-striped"
        }, /*#__PURE__*/node_modules_react.createElement("tbody", null, (this.props.commands || []).map(function (c, i) {
          return /*#__PURE__*/node_modules_react.createElement(Command, CommandGroup_extends({
            key: i
          }, c));
        })));
      }

      var show = null;

      if (this.props.commands.length > 0 && !this.props.modified && this.props.expandable) {
        if (!this.state.expanded) {
          show = /*#__PURE__*/node_modules_react.createElement("button", {
            className: "btn btn-info btn-sm",
            onClick: function onClick() {
              return _this2.toggle(true);
            }
          }, "Show");
        } else {
          show = /*#__PURE__*/node_modules_react.createElement("button", {
            className: "btn btn-info btn-sm",
            onClick: function onClick() {
              return _this2.toggle(false);
            }
          }, "Hide");
        }
      }

      return /*#__PURE__*/node_modules_react.createElement(node_modules_react.Fragment, null, /*#__PURE__*/node_modules_react.createElement("div", {
        className: "oxi-command-group"
      }, /*#__PURE__*/node_modules_react.createElement("div", {
        className: "oxi-command-group-name"
      }, this.props.name), /*#__PURE__*/node_modules_react.createElement("div", {
        className: "oxi-command-group-content"
      }, /*#__PURE__*/node_modules_react.createElement(Content, {
        source: this.props.content
      })), /*#__PURE__*/node_modules_react.createElement("div", {
        className: "oxi-command-group-actions"
      }, show), commands));
    }
  }]);

  return CommandGroup;
}(node_modules_react.Component);


;// CONCATENATED MODULE: ../shared-ui/components/Help.js
function Help_extends() { Help_extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return Help_extends.apply(this, arguments); }

function Help_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Help_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Help_createClass(Constructor, protoProps, staticProps) { if (protoProps) Help_defineProperties(Constructor.prototype, protoProps); if (staticProps) Help_defineProperties(Constructor, staticProps); return Constructor; }

function Help_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Help_setPrototypeOf(subClass, superClass); }

function Help_setPrototypeOf(o, p) { Help_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Help_setPrototypeOf(o, p); }

function Help_createSuper(Derived) { var hasNativeReflectConstruct = Help_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Help_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Help_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Help_possibleConstructorReturn(this, result); }; }

function Help_possibleConstructorReturn(self, call) { if (call && (Help_typeof(call) === "object" || typeof call === "function")) { return call; } return Help_assertThisInitialized(self); }

function Help_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Help_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Help_getPrototypeOf(o) { Help_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Help_getPrototypeOf(o); }

function Help_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Help_typeof = function _typeof(obj) { return typeof obj; }; } else { Help_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Help_typeof(obj); }

function Help_createForOfIteratorHelper(o, allowArrayLike) { var it; if (typeof Symbol === "undefined" || o[Symbol.iterator] == null) { if (Array.isArray(o) || (it = Help_unsupportedIterableToArray(o)) || allowArrayLike && o && typeof o.length === "number") { if (it) o = it; var i = 0; var F = function F() {}; return { s: F, n: function n() { if (i >= o.length) return { done: true }; return { done: false, value: o[i++] }; }, e: function e(_e) { throw _e; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var normalCompletion = true, didErr = false, err; return { s: function s() { it = o[Symbol.iterator](); }, n: function n() { var step = it.next(); normalCompletion = step.done; return step; }, e: function e(_e2) { didErr = true; err = _e2; }, f: function f() { try { if (!normalCompletion && it.return != null) it.return(); } finally { if (didErr) throw err; } } }; }

function Help_unsupportedIterableToArray(o, minLen) { if (!o) return; if (typeof o === "string") return Help_arrayLikeToArray(o, minLen); var n = Object.prototype.toString.call(o).slice(8, -1); if (n === "Object" && o.constructor) n = o.constructor.name; if (n === "Map" || n === "Set") return Array.from(o); if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return Help_arrayLikeToArray(o, minLen); }

function Help_arrayLikeToArray(arr, len) { if (len == null || len > arr.length) len = arr.length; for (var i = 0, arr2 = new Array(len); i < len; i++) { arr2[i] = arr[i]; } return arr2; }




























function hash(s) {
  var out = new Set();

  var _iterator = Help_createForOfIteratorHelper(s.split(/\s+/)),
      _step;

  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var e = _step.value;
      e = e.toLowerCase().replace(/[\s!<>`]+/, '');

      if (e.length === 0) {
        continue;
      }

      out.add(e);
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }

  return out;
}

function matches(test, s) {
  s = hash(s);

  var _iterator2 = Help_createForOfIteratorHelper(test.values()),
      _step2;

  try {
    var _loop = function _loop() {
      var value = _step2.value;

      if (!setAny(s.values(), function (s) {
        return s.startsWith(value);
      })) {
        return {
          v: false
        };
      }
    };

    for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
      var _ret = _loop();

      if (Help_typeof(_ret) === "object") return _ret.v;
    }
  } catch (err) {
    _iterator2.e(err);
  } finally {
    _iterator2.f();
  }

  return true;

  function setAny(values, f) {
    var _iterator3 = Help_createForOfIteratorHelper(values),
        _step3;

    try {
      for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
        var value = _step3.value;

        if (f(value)) {
          return true;
        }
      }
    } catch (err) {
      _iterator3.e(err);
    } finally {
      _iterator3.f();
    }

    return false;
  }
}

var Help = /*#__PURE__*/function (_React$Component) {
  Help_inherits(Help, _React$Component);

  var _super = Help_createSuper(Help);

  function Help(props) {
    var _this;

    Help_classCallCheck(this, Help);

    _this = _super.call(this, props);
    var q = new URLSearchParams(_this.props.location.search);
    _this.state = {
      loading: true,
      groups: props.commands.groups,
      filter: q.get('q') || '',
      groupsLimit: 3
    };
    _this.defaultGroupsLimit = 3;
    return _this;
  }

  Help_createClass(Help, [{
    key: "componentDidMount",
    value: function componentDidMount() {
      this.setState({
        loading: false
      });
    }
  }, {
    key: "filter",
    value: function filter(groups, def) {
      var filter = this.state.filter;

      if (filter === '') {
        return def;
      }

      if (filter.startsWith('!')) {
        groups = groups.map(function (g) {
          var commands = g.commands.filter(function (c) {
            return c.name.startsWith(filter);
          });
          var modified = commands.length != g.commands;
          return Object.assign({}, g, {
            commands: commands,
            modified: modified
          });
        });
      } else {
        var test = hash(filter);
        groups = groups.map(function (g) {
          var commands = g.commands.filter(function (c) {
            return matches(test, c.name);
          });
          var modified = commands.length != g.commands;
          return Object.assign({}, g, {
            commands: commands,
            modified: modified
          });
        });
      }

      return groups.filter(function (g) {
        return g.commands.length > 0;
      });
    }
  }, {
    key: "changeFilter",
    value: function changeFilter(filter) {
      var path = "".concat(this.props.location.pathname);

      if (!!filter) {
        var search = new URLSearchParams(this.props.location.search);
        search.set('q', filter);
        path = "".concat(path, "?").concat(search);
      }

      this.props.history.replace(path);
      this.setState({
        filter: filter,
        groupsLimit: this.defaultGroupsLimit
      });
    }
  }, {
    key: "prevent",
    value: function prevent(e) {
      e.preventDefault();
      return false;
    }
  }, {
    key: "showMore",
    value: function showMore() {
      this.setState({
        groupsLimit: this.state.groupsLimit + 1
      });
    }
  }, {
    key: "showRest",
    value: function showRest() {
      this.setState({
        groupsLimit: null
      });
    }
  }, {
    key: "render",
    value: function render() {
      var _this2 = this;

      var groups = this.filter(this.state.groups, this.state.groups);
      var showMore = null;

      if (this.state.groupsLimit !== null && groups.length > this.state.groupsLimit) {
        var more = groups.length - this.state.groupsLimit;
        groups = groups.slice(0, this.state.groupsLimit);
        showMore = /*#__PURE__*/node_modules_react.createElement("div", {
          className: "mt-3 mb-3 center"
        }, /*#__PURE__*/node_modules_react.createElement("div", {
          className: "btn-group"
        }, /*#__PURE__*/node_modules_react.createElement("button", {
          className: "btn btn-primary btn-lg",
          onClick: function onClick() {
            return _this2.showRest();
          }
        }, "Show More (", more, " more)")));
      }

      var clear = null;

      if (this.state.filter !== '') {
        clear = /*#__PURE__*/node_modules_react.createElement("div", {
          className: "input-group-append"
        }, /*#__PURE__*/node_modules_react.createElement("button", {
          className: "btn btn-danger",
          onClick: function onClick() {
            return _this2.changeFilter('');
          }
        }, "Clear Filter"));
      }

      var toggleShowButton = /*#__PURE__*/node_modules_react.createElement("div", {
        className: "input-group-append"
      }, toggleShowButton);
      var groupsRender = null;

      if (this.state.filter !== '' && groups.length === 0) {
        groupsRender = /*#__PURE__*/node_modules_react.createElement("div", {
          className: "alert alert-warning mt-3 mb-3"
        }, "No documentation matching \"", this.state.filter, "\"");
      } else {
        groupsRender = groups.map(function (c, index) {
          return /*#__PURE__*/node_modules_react.createElement(CommandGroup, Help_extends({
            key: index
          }, c));
        });
      }

      return /*#__PURE__*/node_modules_react.createElement(node_modules_react.Fragment, null, /*#__PURE__*/node_modules_react.createElement("h2", {
        className: "oxi-page-title"
      }, "Command Help"), /*#__PURE__*/node_modules_react.createElement("div", {
        className: "alert alert-info"
      }, /*#__PURE__*/node_modules_react.createElement("b", null, "Want to help expand this page?"), /*#__PURE__*/node_modules_react.createElement("br", null), "You can do that by contributing to the ", /*#__PURE__*/node_modules_react.createElement("a", {
        href: "https://github.com/udoprog/OxidizeBot/blob/master/shared/commands.toml"
      }, /*#__PURE__*/node_modules_react.createElement("code", null, "commands.toml")), " file on Github!"), /*#__PURE__*/node_modules_react.createElement("h4", null, "Search:"), /*#__PURE__*/node_modules_react.createElement("form", {
        onSubmit: this.prevent.bind(this)
      }, /*#__PURE__*/node_modules_react.createElement("div", {
        className: "input-group"
      }, /*#__PURE__*/node_modules_react.createElement("input", {
        className: "form-control",
        placeholder: "type to search...",
        value: this.state.filter || '',
        onChange: function onChange(e) {
          return _this2.changeFilter(e.target.value);
        }
      }), clear, toggleShowButton)), groupsRender, showMore);
    }
  }]);

  return Help;
}(node_modules_react.Component);


;// CONCATENATED MODULE: ./src/components/Help.js
function components_Help_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { components_Help_typeof = function _typeof(obj) { return typeof obj; }; } else { components_Help_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return components_Help_typeof(obj); }













function components_Help_extends() { components_Help_extends = Object.assign || function (target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i]; for (var key in source) { if (Object.prototype.hasOwnProperty.call(source, key)) { target[key] = source[key]; } } } return target; }; return components_Help_extends.apply(this, arguments); }

function components_Help_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function components_Help_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function components_Help_createClass(Constructor, protoProps, staticProps) { if (protoProps) components_Help_defineProperties(Constructor.prototype, protoProps); if (staticProps) components_Help_defineProperties(Constructor, staticProps); return Constructor; }

function components_Help_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) components_Help_setPrototypeOf(subClass, superClass); }

function components_Help_setPrototypeOf(o, p) { components_Help_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return components_Help_setPrototypeOf(o, p); }

function components_Help_createSuper(Derived) { var hasNativeReflectConstruct = components_Help_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = components_Help_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = components_Help_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return components_Help_possibleConstructorReturn(this, result); }; }

function components_Help_possibleConstructorReturn(self, call) { if (call && (components_Help_typeof(call) === "object" || typeof call === "function")) { return call; } return components_Help_assertThisInitialized(self); }

function components_Help_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function components_Help_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function components_Help_getPrototypeOf(o) { components_Help_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return components_Help_getPrototypeOf(o); }






var HelpPage = /*#__PURE__*/function (_React$Component) {
  components_Help_inherits(HelpPage, _React$Component);

  var _super = components_Help_createSuper(HelpPage);

  function HelpPage(props) {
    components_Help_classCallCheck(this, HelpPage);

    return _super.call(this, props);
  }

  components_Help_createClass(HelpPage, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement(Help, components_Help_extends({
        commands: (commands_default())
      }, this.props)));
    }
  }]);

  return HelpPage;
}(react.Component);


;// CONCATENATED MODULE: ./src/components/Privacy.js
function Privacy_typeof(obj) { "@babel/helpers - typeof"; if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { Privacy_typeof = function _typeof(obj) { return typeof obj; }; } else { Privacy_typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return Privacy_typeof(obj); }












function Privacy_classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

function Privacy_defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } }

function Privacy_createClass(Constructor, protoProps, staticProps) { if (protoProps) Privacy_defineProperties(Constructor.prototype, protoProps); if (staticProps) Privacy_defineProperties(Constructor, staticProps); return Constructor; }

function Privacy_inherits(subClass, superClass) { if (typeof superClass !== "function" && superClass !== null) { throw new TypeError("Super expression must either be null or a function"); } subClass.prototype = Object.create(superClass && superClass.prototype, { constructor: { value: subClass, writable: true, configurable: true } }); if (superClass) Privacy_setPrototypeOf(subClass, superClass); }

function Privacy_setPrototypeOf(o, p) { Privacy_setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) { o.__proto__ = p; return o; }; return Privacy_setPrototypeOf(o, p); }

function Privacy_createSuper(Derived) { var hasNativeReflectConstruct = Privacy_isNativeReflectConstruct(); return function _createSuperInternal() { var Super = Privacy_getPrototypeOf(Derived), result; if (hasNativeReflectConstruct) { var NewTarget = Privacy_getPrototypeOf(this).constructor; result = Reflect.construct(Super, arguments, NewTarget); } else { result = Super.apply(this, arguments); } return Privacy_possibleConstructorReturn(this, result); }; }

function Privacy_possibleConstructorReturn(self, call) { if (call && (Privacy_typeof(call) === "object" || typeof call === "function")) { return call; } return Privacy_assertThisInitialized(self); }

function Privacy_assertThisInitialized(self) { if (self === void 0) { throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); } return self; }

function Privacy_isNativeReflectConstruct() { if (typeof Reflect === "undefined" || !Reflect.construct) return false; if (Reflect.construct.sham) return false; if (typeof Proxy === "function") return true; try { Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); return true; } catch (e) { return false; } }

function Privacy_getPrototypeOf(o) { Privacy_getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) { return o.__proto__ || Object.getPrototypeOf(o); }; return Privacy_getPrototypeOf(o); }





var Privacy = /*#__PURE__*/function (_React$Component) {
  Privacy_inherits(Privacy, _React$Component);

  var _super = Privacy_createSuper(Privacy);

  function Privacy(props) {
    Privacy_classCallCheck(this, Privacy);

    return _super.call(this, props);
  }

  Privacy_createClass(Privacy, [{
    key: "render",
    value: function render() {
      return /*#__PURE__*/react.createElement(RouteLayout, null, /*#__PURE__*/react.createElement("h1", {
        className: "oxi-page-title"
      }, "Privacy Policy"), /*#__PURE__*/react.createElement("p", null, "Effective date: October 4, 2019"), /*#__PURE__*/react.createElement("p", null, "setbac.tv (\"us\", \"we\", or \"our\") operates the OxidizeBot Desktop application and Service  (the \"Service\")."), /*#__PURE__*/react.createElement("p", null, "This page informs you of our policies regarding the collection, use, and disclosure of personal data when you use our Service and the choices you have associated with that data."), /*#__PURE__*/react.createElement("p", null, /*#__PURE__*/react.createElement("b", null, "We don't collect any personal data about our users."), " This service will only ever store OAuth 2.0 access tokens which are made available to the OxidizeBot desktop application at your request."), /*#__PURE__*/react.createElement("p", null, "At any time, you can revoke this consent under ", /*#__PURE__*/react.createElement(react_router_dom/* Link */.rU, {
        to: "/connections"
      }, "My Connections"), ". After which ", /*#__PURE__*/react.createElement("em", null, "all data"), " associated with the connection will be deleted."), /*#__PURE__*/react.createElement("h2", null, "Changes To This Privacy Policy"), /*#__PURE__*/react.createElement("p", null, "We may update our Privacy Policy from time to time. We will notify you of any changes by posting the new Privacy Policy on this page."), /*#__PURE__*/react.createElement("p", null, "We will let you know via email and/or a prominent notice on our Service, prior to the change becoming effective and update the \"effective date\" at the top of this Privacy Policy."), /*#__PURE__*/react.createElement("p", null, "You are advised to review this Privacy Policy periodically for any changes. Changes to this Privacy Policy are effective when they are posted on this page."), /*#__PURE__*/react.createElement("h2", null, "Contact Us"), /*#__PURE__*/react.createElement("p", null, "If you have any questions about this Privacy Policy, please contact us:"), /*#__PURE__*/react.createElement("ul", null, /*#__PURE__*/react.createElement("li", null, "By email: ", /*#__PURE__*/react.createElement("a", {
        href: "mailto:udoprog@tedro.se"
      }, "udoprog@tedro.se"))));
    }
  }]);

  return Privacy;
}(react.Component);


// EXTERNAL MODULE: ./node_modules/@fortawesome/fontawesome-svg-core/index.es.js
var fontawesome_svg_core_index_es = __webpack_require__(8947);
// EXTERNAL MODULE: ./node_modules/@fortawesome/free-solid-svg-icons/index.es.js
var free_solid_svg_icons_index_es = __webpack_require__(1436);
// EXTERNAL MODULE: ./node_modules/@fortawesome/free-brands-svg-icons/index.es.js
var free_brands_svg_icons_index_es = __webpack_require__(1417);
;// CONCATENATED MODULE: ./src/index.js













fontawesome_svg_core_index_es/* library.add */.vI.add(free_solid_svg_icons_index_es/* faQuestion */.Psp, free_solid_svg_icons_index_es/* faGlobe */.g4A, free_solid_svg_icons_index_es/* faCopy */.kZ_, free_solid_svg_icons_index_es/* faSignOutAlt */.jLD, free_solid_svg_icons_index_es/* faEyeSlash */.Aq, free_solid_svg_icons_index_es/* faEye */.Mdf, free_solid_svg_icons_index_es/* faShare */.zBy, free_solid_svg_icons_index_es/* faHome */.J9Y, free_solid_svg_icons_index_es/* faMusic */.Xig, free_solid_svg_icons_index_es/* faTrash */.$aW, free_solid_svg_icons_index_es/* faCheck */.LEp, free_solid_svg_icons_index_es/* faSync */.UO1, free_solid_svg_icons_index_es/* faPlug */.oso);

fontawesome_svg_core_index_es/* library.add */.vI.add(free_brands_svg_icons_index_es/* faTwitch */.z0T, free_brands_svg_icons_index_es/* faYoutube */.opf, free_brands_svg_icons_index_es/* faSpotify */.Ha7, free_brands_svg_icons_index_es/* faTwitter */.mdU, free_brands_svg_icons_index_es/* faGithub */.zhw);

function AppRouter() {
  return /*#__PURE__*/react.createElement(react_router_dom/* BrowserRouter */.VK, null, /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/",
    exact: true,
    component: Index
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/playlists",
    exact: true,
    component: Players
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/player/:id",
    exact: true,
    component: Player
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/connections",
    exact: true,
    component: Connections
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/help",
    exact: true,
    component: HelpPage
  }), /*#__PURE__*/react.createElement(react_router/* Route */.AW, {
    path: "/privacy",
    exact: true,
    component: Privacy
  }));
}

updateGlobals().then(function () {
  return react_dom.render( /*#__PURE__*/react.createElement(AppRouter, null), document.getElementById("index"));
});

/***/ })

/******/ 	});
/************************************************************************/
/******/ 	// The module cache
/******/ 	var __webpack_module_cache__ = {};
/******/ 	
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/ 		// Check if module is in cache
/******/ 		if(__webpack_module_cache__[moduleId]) {
/******/ 			return __webpack_module_cache__[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = __webpack_module_cache__[moduleId] = {
/******/ 			// no module.id needed
/******/ 			// no module.loaded needed
/******/ 			exports: {}
/******/ 		};
/******/ 	
/******/ 		// Execute the module function
/******/ 		__webpack_modules__[moduleId](module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = __webpack_modules__;
/******/ 	
/******/ 	// the startup function
/******/ 	// It's empty as some runtime module handles the default behavior
/******/ 	__webpack_require__.x = x => {};
/************************************************************************/
/******/ 	/* webpack/runtime/compat get default export */
/******/ 	(() => {
/******/ 		// getDefaultExport function for compatibility with non-harmony modules
/******/ 		__webpack_require__.n = (module) => {
/******/ 			var getter = module && module.__esModule ?
/******/ 				() => (module['default']) :
/******/ 				() => (module);
/******/ 			__webpack_require__.d(getter, { a: getter });
/******/ 			return getter;
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/define property getters */
/******/ 	(() => {
/******/ 		// define getter functions for harmony exports
/******/ 		__webpack_require__.d = (exports, definition) => {
/******/ 			for(var key in definition) {
/******/ 				if(__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
/******/ 					Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
/******/ 				}
/******/ 			}
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/global */
/******/ 	(() => {
/******/ 		__webpack_require__.g = (function() {
/******/ 			if (typeof globalThis === 'object') return globalThis;
/******/ 			try {
/******/ 				return this || new Function('return this')();
/******/ 			} catch (e) {
/******/ 				if (typeof window === 'object') return window;
/******/ 			}
/******/ 		})();
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/hasOwnProperty shorthand */
/******/ 	(() => {
/******/ 		__webpack_require__.o = (obj, prop) => (Object.prototype.hasOwnProperty.call(obj, prop))
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/make namespace object */
/******/ 	(() => {
/******/ 		// define __esModule on exports
/******/ 		__webpack_require__.r = (exports) => {
/******/ 			if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
/******/ 				Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 			}
/******/ 			Object.defineProperty(exports, '__esModule', { value: true });
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/publicPath */
/******/ 	(() => {
/******/ 		var scriptUrl;
/******/ 		if (__webpack_require__.g.importScripts) scriptUrl = __webpack_require__.g.location + "";
/******/ 		var document = __webpack_require__.g.document;
/******/ 		if (!scriptUrl && document) {
/******/ 			if (document.currentScript)
/******/ 				scriptUrl = document.currentScript.src
/******/ 			if (!scriptUrl) {
/******/ 				var scripts = document.getElementsByTagName("script");
/******/ 				if(scripts.length) scriptUrl = scripts[scripts.length - 1].src
/******/ 			}
/******/ 		}
/******/ 		// When supporting browsers where an automatic publicPath is not supported you must specify an output.publicPath manually via configuration
/******/ 		// or pass an empty string ("") and set the __webpack_public_path__ variable from your code to use your own logic.
/******/ 		if (!scriptUrl) throw new Error("Automatic publicPath is not supported in this browser");
/******/ 		scriptUrl = scriptUrl.replace(/#.*$/, "").replace(/\?.*$/, "").replace(/\/[^\/]+$/, "/");
/******/ 		__webpack_require__.p = scriptUrl;
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/jsonp chunk loading */
/******/ 	(() => {
/******/ 		// no baseURI
/******/ 		
/******/ 		// object to store loaded and loading chunks
/******/ 		// undefined = chunk not loaded, null = chunk preloaded/prefetched
/******/ 		// Promise = chunk loading, 0 = chunk loaded
/******/ 		var installedChunks = {
/******/ 			179: 0
/******/ 		};
/******/ 		
/******/ 		var deferredModules = [
/******/ 			[6469,511]
/******/ 		];
/******/ 		// no chunk on demand loading
/******/ 		
/******/ 		// no prefetching
/******/ 		
/******/ 		// no preloaded
/******/ 		
/******/ 		// no HMR
/******/ 		
/******/ 		// no HMR manifest
/******/ 		
/******/ 		var checkDeferredModules = x => {};
/******/ 		
/******/ 		// install a JSONP callback for chunk loading
/******/ 		var webpackJsonpCallback = (parentChunkLoadingFunction, data) => {
/******/ 			var [chunkIds, moreModules, runtime, executeModules] = data;
/******/ 			// add "moreModules" to the modules object,
/******/ 			// then flag all "chunkIds" as loaded and fire callback
/******/ 			var moduleId, chunkId, i = 0, resolves = [];
/******/ 			for(;i < chunkIds.length; i++) {
/******/ 				chunkId = chunkIds[i];
/******/ 				if(__webpack_require__.o(installedChunks, chunkId) && installedChunks[chunkId]) {
/******/ 					resolves.push(installedChunks[chunkId][0]);
/******/ 				}
/******/ 				installedChunks[chunkId] = 0;
/******/ 			}
/******/ 			for(moduleId in moreModules) {
/******/ 				if(__webpack_require__.o(moreModules, moduleId)) {
/******/ 					__webpack_require__.m[moduleId] = moreModules[moduleId];
/******/ 				}
/******/ 			}
/******/ 			if(runtime) runtime(__webpack_require__);
/******/ 			if(parentChunkLoadingFunction) parentChunkLoadingFunction(data);
/******/ 			while(resolves.length) {
/******/ 				resolves.shift()();
/******/ 			}
/******/ 		
/******/ 			// add entry modules from loaded chunk to deferred list
/******/ 			if(executeModules) deferredModules.push.apply(deferredModules, executeModules);
/******/ 		
/******/ 			// run deferred modules when all chunks ready
/******/ 			return checkDeferredModules();
/******/ 		}
/******/ 		
/******/ 		var chunkLoadingGlobal = self["webpackChunkweb"] = self["webpackChunkweb"] || [];
/******/ 		chunkLoadingGlobal.forEach(webpackJsonpCallback.bind(null, 0));
/******/ 		chunkLoadingGlobal.push = webpackJsonpCallback.bind(null, chunkLoadingGlobal.push.bind(chunkLoadingGlobal));
/******/ 		
/******/ 		function checkDeferredModulesImpl() {
/******/ 			var result;
/******/ 			for(var i = 0; i < deferredModules.length; i++) {
/******/ 				var deferredModule = deferredModules[i];
/******/ 				var fulfilled = true;
/******/ 				for(var j = 1; j < deferredModule.length; j++) {
/******/ 					var depId = deferredModule[j];
/******/ 					if(installedChunks[depId] !== 0) fulfilled = false;
/******/ 				}
/******/ 				if(fulfilled) {
/******/ 					deferredModules.splice(i--, 1);
/******/ 					result = __webpack_require__(__webpack_require__.s = deferredModule[0]);
/******/ 				}
/******/ 			}
/******/ 			if(deferredModules.length === 0) {
/******/ 				__webpack_require__.x();
/******/ 				__webpack_require__.x = x => {};
/******/ 			}
/******/ 			return result;
/******/ 		}
/******/ 		var startup = __webpack_require__.x;
/******/ 		__webpack_require__.x = () => {
/******/ 			// reset startup function so it can be called again when more startup code is added
/******/ 			__webpack_require__.x = startup || (x => {});
/******/ 			return (checkDeferredModules = checkDeferredModulesImpl)();
/******/ 		};
/******/ 	})();
/******/ 	
/************************************************************************/
/******/ 	
/******/ 	// run startup
/******/ 	var __webpack_exports__ = __webpack_require__.x();
/******/ 	
/******/ })()
;