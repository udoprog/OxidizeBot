use crate::{api, command, config, db, idle, injector, irc, settings, stream_info, utils};
use hashbrown::HashMap;
use std::sync::Arc;

#[macro_use]
mod macros;
pub mod admin;
pub mod after_stream;
pub mod alias_admin;
pub mod auth;
pub mod clip;
pub mod command_admin;
pub mod countdown;
pub mod eight_ball;
pub mod gtav;
pub mod misc;
pub mod promotions;
pub mod song;
pub mod speedrun;
pub mod swearjar;
pub mod theme_admin;
pub mod time;
pub mod water;

#[derive(Default)]
pub struct Handlers<'a> {
    handlers: HashMap<String, Box<dyn command::Handler + Send + 'a>>,
}

impl<'a> Handlers<'a> {
    /// Insert the given handler.
    pub fn insert(&mut self, command: impl AsRef<str>, handler: impl command::Handler + Send + 'a) {
        self.handlers
            .insert(command.as_ref().to_string(), Box::new(handler));
    }

    /// Lookup the given command mutably.
    pub fn get_mut(&mut self, command: &str) -> Option<&mut (dyn command::Handler + Send + 'a)> {
        self.handlers.get_mut(command).map(|command| &mut **command)
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Config {
    #[serde(rename = "countdown")]
    Countdown(countdown::Config),
    #[serde(rename = "swearjar")]
    SwearJar(swearjar::Config),
    #[serde(rename = "water")]
    Water(water::Config),
    #[serde(rename = "promotions")]
    Promotions(promotions::Config),
    #[serde(rename = "gtav")]
    Gtav(gtav::Config),
}

/// Context for a hook.
pub struct HookContext<'a: 'm, 'm> {
    pub handlers: &'m mut Handlers<'a>,
    pub futures: &'m mut utils::Futures<'a>,
    pub stream_info: &'m stream_info::StreamInfo,
    pub idle: &'m idle::Idle,
    pub injector: &'m injector::Injector,
    pub config: &'a config::Config,
    pub db: &'a db::Database,
    pub commands: &'a db::Commands,
    pub aliases: &'a db::Aliases,
    pub promotions: &'a db::Promotions,
    pub themes: &'a db::Themes,
    pub after_streams: &'a db::AfterStreams,
    pub youtube: &'a Arc<api::YouTube>,
    pub twitch: &'a api::Twitch,
    pub streamer_twitch: &'a api::Twitch,
    pub sender: &'a irc::Sender,
    pub settings: &'a settings::Settings,
    pub auth: &'a crate::auth::Auth,
}

pub trait Module: Send + 'static {
    /// Type of the module as a string to help with diagnostics.
    fn ty(&self) -> &'static str;

    /// Set up command handlers for this module.
    fn hook(&self, _: HookContext<'_, '_>) -> Result<(), failure::Error> {
        Ok(())
    }
}
