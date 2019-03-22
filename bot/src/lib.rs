#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod aliases;
mod command;
pub mod config;
pub mod currency;
mod current_song;
pub mod db;
pub mod features;
pub mod irc;
mod module;
pub mod oauth2;
pub mod player;
pub mod secrets;
pub mod setbac;
pub mod spotify;
mod template;
mod themes;
mod track_id;
pub mod twitch;
mod utils;
pub mod web;
