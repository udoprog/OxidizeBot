<a href="https://setbac.tv">
    <img src="https://raw.githubusercontent.com/udoprog/OxidizeBot/main/bot/res/icon48.png" title="Oxidize Bot">
</a>
<br>
<a href="https://setbac.tv/" rel="nofollow">Site 🌐</a>
&ndash;
<a href="https://setbac.tv/help" rel="nofollow">Command Help ❓</a>

# oxidize
[<img alt="github" src="https://img.shields.io/badge/github-udoprog/OxidizeBot-8da0cb?style=for-the-badge&logo=github" height="24">](https://github.com/udoprog/OxidizeBot)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/udoprog/OxidizeBot/ci.yml?branch=main&style=for-the-badge" height="24">](https://github.com/udoprog/OxidizeBot/actions?query=branch%3Amain)
[<img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="24">](https://discord.gg/v5AeNkT)

A high performance Twitch Bot powered by Rust.


<br>

## Features

**Commands** &mdash; Aliases, custom commands, promotions, plus [a bunch more](https://setbac.tv/help).

If there's something you're missing, feel free to [open an issue].

**Rust** &mdash; Written in [Rust], promoting high performance, low utilization, and reliability.

<p>
<img style="float: left;"  title="Rust" width="67" height="50" src="https://github.com/udoprog/OxidizeBot/raw/main/gfx/cuddlyferris.png" />
</p>

**Configurable** &mdash; Everything is tweakable to suit your needs through
a [hundred settings]. Changes to settings applies immediately - no need to
restart.

<p>
<img style="float: left;" title="Settings" width="140" height="50" src="https://github.com/udoprog/OxidizeBot/raw/main/gfx/setting.png" />
</p>

**Integrated with Windows** &mdash; Runs in the background with a System
Tray. Notifies you on issues. Starts automatically with Windows if you want
it to.

<p>
<img style="float: left;" title="Windows Systray" width="131" height="50" src="https://github.com/udoprog/OxidizeBot/raw/main/gfx/windows-systray.png" />
<img style="float: left;" title="Reminder" width="120" height="50" src="https://github.com/udoprog/OxidizeBot/raw/main/gfx/windows-reminder.png" />
</p>

<br>

## Installing and Running

You can download an installer or an archive from [releases] or [build the project yourself](#building).

[releases]: https://github.com/udoprog/OxidizeBot/releases

<br>

## Building

You'll need Rust and a working compiler: <https://rustup.rs/>.

After this, you build the project using cargo:

```bash
cargo build
```

If you want to run it directly from the project directory, you can do:

```bash
cargo run
```

If you want to run the bot with the most amount of diagnostics possible, you can
do the following:

```bash
env RUST_BACKTRACE=1 cargo +nightly run -- --log oxidize=trace
```

This will include backtraces on errors, which is currently an [unstable feature].

[unstable feature]: https://doc.rust-lang.org/std/backtrace/index.html

<br>

## License

OxidizeBot is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE], [LICENSE-MIT] for details.

[open an issue]: https://github.com/udoprog/OxidizeBot/issues
[Rust]: https://rust-lang.org
[hundred settings]: https://github.com/udoprog/OxidizeBot/blob/main/bot/src/settings.yaml
[LICENSE-APACHE]: https://github.com/udoprog/OxidizeBot/blob/main/LICENSE-APACHE
[LICENSE-MIT]: https://github.com/udoprog/OxidizeBot/blob/main/LICENSE-MIT
