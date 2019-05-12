#![recursion_limit = "128"]

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
pub mod bus;
mod command;
pub mod config;
pub mod currency;
mod current_song;
pub mod db;
pub mod features;
mod idle;
pub mod irc;
pub mod module;
pub mod oauth2;
pub mod player;
pub mod secrets;
pub mod settings;
mod spotify_id;
mod stream_info;
pub mod template;
mod themes;
mod track_id;
pub mod utils;
pub mod web;
