#![feature(async_await)]
#![feature(arbitrary_self_types)]
#![recursion_limit = "512"]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate warp;

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[macro_use]
mod macros;
pub mod api;
pub mod auth;
pub mod bus;
mod command;
pub mod config;
pub mod currency;
pub mod db;
pub mod features;
mod idle;
pub mod injector;
pub mod irc;
pub mod module;
pub mod oauth2;
pub mod obs;
pub mod player;
pub mod prelude;
pub mod secrets;
pub mod settings;
mod song_file;
mod spotify_id;
mod stream_info;
pub mod template;
mod timer;
mod track_id;
pub mod utils;
pub mod web;
