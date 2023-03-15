//! <a href="https://setbac.tv">
//!     <img src="https://raw.githubusercontent.com/udoprog/OxidizeBot/main/bot/res/icon48.png" title="Oxidize Bot">
//! </a>
//!
//! <br>
//!
//! <p align="center">
//!     A high performance Twitch Bot powered by Rust
//! </p>
//!
//! <div align="center">
//!     <a href="https://github.com/udoprog/OxidizeBot"><img alt="github" src="https://img.shields.io/badge/github-udoprog/OxidizeBot-8da0cb?style=for-the-badge&logo=github" height="24"></a>
//!     <a href="https://github.com/udoprog/OxidizeBot/actions?query=branch%3Amain"><img alt="build status" src="https://img.shields.io/github/actions/workflow/status/udoprog/OxidizeBot/ci.yml?branch=main&style=for-the-badge" height="24"></a>
//!     <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="24"></a>
//! </div>
//!
//! <div align="center">
//!     <a href="https://setbac.tv/" rel="nofollow">Site 🌐</a>
//!     &ndash;
//!     <a href="https://setbac.tv/help" rel="nofollow">Command Help ❓</a>
//! </div>
//!
//! <br>
//!
//! ## Features
//!
//! **Commands** &mdash; Aliases, custom commands, promotions, plus [a bunch more](https://setbac.tv/help).
//!
//! If there's something you're missing, feel free to [open an issue].
//!
//! **Rust** &mdash; Written in [Rust], promoting high performance, low utilization, and reliability.
//!
//! <p>
//! <img style="float: left;"  title="Rust" width="67" height="50" src="https://github.com/udoprog/OxidizeBot/raw/main/gfx/cuddlyferris.png" />
//! </p>
//!
//! **Configurable** &mdash; Everything is tweakable to suit your needs through
//! a [hundred settings]. Changes to settings applies immediately - no need to
//! restart.
//!
//! <p>
//! <img style="float: left;" title="Settings" width="140" height="50" src="https://github.com/udoprog/OxidizeBot/raw/main/gfx/setting.png" />
//! </p>
//!
//! **Integrated with Windows** &mdash; Runs in the background with a System
//! Tray. Notifies you on issues. Starts automatically with Windows if you want
//! it to.
//!
//! <p>
//! <img style="float: left;" title="Windows Systray" width="131" height="50" src="https://github.com/udoprog/OxidizeBot/raw/main/gfx/windows-systray.png" />
//! <img style="float: left;" title="Reminder" width="120" height="50" src="https://github.com/udoprog/OxidizeBot/raw/main/gfx/windows-reminder.png" />
//! </p>
//!
//! <br>
//!
//! ## Installing and Running
//!
//! You can download an installer or an archive from [releases] or [build the project yourself](#building).
//!
//! [releases]: https://github.com/udoprog/OxidizeBot/releases
//!
//! <br>
//!
//! ## Building
//!
//! You'll need Rust and a working compiler: https://rustup.rs/
//!
//! After this, you build the project using cargo:
//!
//! ```bash
//! cargo build
//! ```
//!
//! If you want to run it directly from the project directory, you can do:
//!
//! ```bash
//! cargo run
//! ```
//!
//! If you want to run the bot with the most amount of diagnostics possible, you can
//! do the following:
//!
//! ```bash
//! env RUST_BACKTRACE=1 cargo +nightly run -- --log oxidize=trace
//! ```
//!
//! This will include backtraces on errors, which is currently an [unstable feature].
//!
//! [unstable feature]: https://doc.rust-lang.org/std/backtrace/index.html
//!
//! <br>
//!
//! ## License
//!
//! OxidizeBot is distributed under the terms of both the MIT license and the
//! Apache License (Version 2.0).
//!
//! See [LICENSE-APACHE], [LICENSE-MIT] for details.
//!
//! [open an issue]: https://github.com/udoprog/OxidizeBot/issues
//! [Rust]: https://rust-lang.org
//! [hundred settings]: https://github.com/udoprog/OxidizeBot/blob/main/bot/src/settings.yaml
//! [LICENSE-APACHE]: https://github.com/udoprog/OxidizeBot/blob/main/LICENSE-APACHE
//! [LICENSE-MIT]: https://github.com/udoprog/OxidizeBot/blob/main/LICENSE-MIT

#![recursion_limit = "1024"]
#![cfg_attr(backtrace, feature(backtrace))]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate smallvec;

pub use async_injector as injector;

/// Get the version number of the project.
macro_rules! version_str {
    () => {
        include_str!(concat!(env!("OUT_DIR"), "/version.txt"))
    };
}

/// Get the user agent.
macro_rules! user_agent_str {
    () => {
        include_str!(concat!(env!("OUT_DIR"), "/user_agent.txt"))
    };
}

pub const VERSION: &str = version_str!();

#[macro_use]
mod macros;
pub mod api;
pub mod auth;
mod backoff;
pub mod bus;
mod command;
pub mod currency;
pub mod db;
pub mod emotes;
mod idle;
pub mod irc;
pub mod message_log;
pub mod module;
pub mod oauth2;
mod panic_logger;
pub mod player;
pub(crate) mod prelude;
#[cfg(feature = "scripting")]
mod script;
#[cfg(not(feature = "scripting"))]
#[path = "script/mock.rs"]
mod script;
pub mod settings;
mod song_file;
mod spotify_id;
pub mod storage;
pub mod stream_info;
pub mod sys;
pub mod tags;
mod task;
pub mod template;
pub mod tracing_utils;
mod track_id;
pub mod updater;
mod uri;
pub mod utils;
pub mod web;
pub(crate) use tokio_stream as stream;

pub use self::panic_logger::panic_logger;
use self::uri::Uri;

/// The local schema alias.
pub(crate) type Schema = crate::settings::Schema<crate::auth::Scope>;
/// The local settings alias.
pub type Settings = crate::settings::Settings<crate::auth::Scope>;
/// The local setting alias.
pub(crate) type Setting = crate::settings::Setting<crate::auth::Scope>;

pub const SCHEMA: &[u8] = include_bytes!("settings.yaml");

/// Load the settings schema to use.
pub fn load_schema() -> Result<crate::Schema, crate::settings::Error> {
    crate::settings::Schema::load_bytes(SCHEMA)
}
