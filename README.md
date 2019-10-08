<p align="center">
  <a href="https://setbac.tv"><img src="https://raw.githubusercontent.com/udoprog/OxidizeBot/master/bot/res/icon48.png" title="Oxidize Bot"></a>
</p>

<p align="center">
  A high performance Twitch Bot powered by Rust
</p>

<p align="center">
  <a href="https://travis-ci.org/udoprog/OxidizeBot">
    <img alt="Build Status" src="https://travis-ci.org/udoprog/OxidizeBot.svg?branch=master">
  </a>

  <a href="https://ci.appveyor.com/project/udoprog/OxidizeBot/branch/master">
    <img alt="Windows Build Status" src="https://ci.appveyor.com/api/projects/status/cxagsq3idti252a4/branch/master?svg=true">
  </a>
</p>

<p align="center">
  <a href="https://setbac.tv/" rel="nofollow">Website</a>
</p>

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

You can download an installer or an archive from [releases](https://github.com/udoprog/OxidizeBot/releases) or [build the project yourself](#building).

## Building

You'll need Rust and a working compiler: https://rustup.rs/

For now and until `async_await` is stable, you will need to use the _beta_ Rust compiler.
This can be installed and configured by running:

```
rustup toolchain install beta
rustup default beta
```

After this, you build the project using cargo:

```
cargo +beta build --release
```

## License

OxidizeBot is primarily distributed under the terms of both the MIT license
and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for details.