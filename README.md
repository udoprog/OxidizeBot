<div align="center">
  <a href="https://setbac.tv">
    <img src="https://raw.githubusercontent.com/udoprog/OxidizeBot/master/bot/res/icon48.png" title="Oxidize Bot">
  </a>
</div>

<p align="center">
  A high performance Twitch Bot powered by Rust
</p>

<div align="center">
  <a href="https://github.com/udoprog/OxidizeBot/actions">
    <img alt="GitHub Actions Build Status" src="https://github.com/udoprog/OxidizeBot/workflows/Build/badge.svg">
  </a>

  <a href="https://discord.gg/v5AeNkT">
    <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
  </a>
</div>

<div align="center">
  <a href="https://setbac.tv/" rel="nofollow">Site 🌐</a>
  &ndash;
  <a href="https://setbac.tv/help" rel="nofollow">Command Help ❓</a>
</div>

## Features

**Commands** &mdash; Aliases, custom commands, promotions, plus [a bunch more](https://setbac.tv/help).

If there's something you're missing, feel free to [open an issue].

**Rust** &mdash; Written in [Rust], promoting high performance, low utilization, and reliability.

<p>
<img style="float: left;"  title="Rust" width="67" height="50" src="https://github.com/udoprog/OxidizeBot/raw/master/gfx/cuddlyferris.png" />
</p>

**Configurable** &mdash; Everything is tweakable to suit your needs through a [hundred settings].
Changes to settings applies immediately - no need to restart.

<p>
<img style="float: left;" title="Settings" width="140" height="50" src="https://github.com/udoprog/OxidizeBot/raw/master/gfx/setting.png" />
</p>

**Integrated with Windows** &mdash; Runs in the background with a System Tray.
Notifies you on issues.
Starts automatically with Windows if you want it to.

<p>
<img style="float: left;" title="Windows Systray" width="131" height="50" src="https://github.com/udoprog/OxidizeBot/raw/master/gfx/windows-systray.png" />
<img style="float: left;" title="Reminder" width="120" height="50" src="https://github.com/udoprog/OxidizeBot/raw/master/gfx/windows-reminder.png" />
</p>

[open an issue]: https://github.com/udoprog/OxidizeBot/issues
[Rust]: https://rust-lang.org
[hundred settings]: /bot/src/settings.yaml

## Installing and Running

You can download an installer or an archive from [releases] or [build the project yourself](#building).

[releases]: https://github.com/udoprog/OxidizeBot/releases

## Building

You'll need Rust and a working compiler: https://rustup.rs/

After this, you build the project using cargo:

```
cargo --manifest-path=bot/Cargo.toml build --release
```

If you want to run it directly from the project directory, you can do:

```
cargo --manifest-path=bot/Cargo.toml run --release --no-default-features
```

Note: `--no-default-features` disables the windows_subsystem configuration on
Windows, allowing you to run the project in the terminal.

If you want to run the bot with the most amount of diagnostics possible, you can
do the following:

```
cargo +nightly --manifest-path=bot/Cargo.toml run --release --no-default-features --features nightly -- --log oxidize=trace
```

This will include backtraces on errors, which is currently an [unstable feature].

[unstable feature]: https://doc.rust-lang.org/std/backtrace/index.html

## License

OxidizeBot is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for details.