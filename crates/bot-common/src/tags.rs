//! Tags used in the OxidizeBot injector.

use serde::Serialize;

/// Identifies the kind of token associated with a connection.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Token {
    Twitch(Twitch),
    YouTube,
    NightBot,
    Spotify,
}

/// Identifies a kind of twitch client.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Twitch {
    Streamer,
    Bot,
}

/// Identifiers a set of generic global variables.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Globals {
    Channel,
}
