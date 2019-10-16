#![recursion_limit = "512"]
#![cfg_attr(feature = "nightly", feature(backtrace))]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate smallvec;

pub use async_injector as injector;

pub const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

#[macro_use]
mod macros;
pub mod api;
pub mod auth;
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
pub mod settings;
mod song_file;
mod spotify_id;
pub mod storage;
pub mod stream_info;
pub mod sys;
pub mod template;
pub mod tracing_utils;
mod track_id;
pub mod updater;
mod uri;
pub mod utils;
pub mod web;

pub use self::panic_logger::panic_logger;
use self::uri::Uri;
