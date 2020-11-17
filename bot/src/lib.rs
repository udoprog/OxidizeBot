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
pub mod prelude;
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
mod task;
pub mod template;
pub mod tracing_utils;
mod track_id;
pub mod updater;
mod uri;
pub mod utils;
pub mod web;

pub use self::panic_logger::panic_logger;
use self::uri::Uri;
