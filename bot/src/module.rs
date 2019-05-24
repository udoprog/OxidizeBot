use crate::{
    api, command, config, currency, db, idle, irc, obs, player, prelude::*, settings, stream_info,
};
use hashbrown::HashMap;
use std::sync::Arc;

#[macro_use]
mod macros;
pub mod admin;
pub mod alias_admin;
pub mod command_admin;
mod countdown;
mod gtav;
mod promotions;
pub mod song;
mod swearjar;
pub mod theme_admin;
mod water;

#[derive(Default)]
pub struct Handlers {
    handlers: HashMap<String, Box<dyn command::Handler + Send + 'static>>,
}

impl Handlers {
    /// Insert the given handler.
    pub fn insert(
        &mut self,
        command: impl AsRef<str>,
        handler: impl command::Handler + Send + 'static,
    ) {
        self.handlers
            .insert(command.as_ref().to_string(), Box::new(handler));
    }

    /// Lookup the given command mutably.
    pub fn get_mut(
        &mut self,
        command: &str,
    ) -> Option<&mut (dyn command::Handler + Send + 'static)> {
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
pub struct HookContext<'a> {
    pub config: &'a config::Config,
    pub db: &'a db::Database,
    pub commands: &'a db::Commands,
    pub aliases: &'a db::Aliases,
    pub promotions: &'a db::Promotions,
    pub themes: &'a db::Themes,
    pub handlers: &'a mut Handlers,
    pub currency: Option<&'a currency::Currency>,
    pub youtube: &'a Arc<api::YouTube>,
    pub twitch: &'a api::Twitch,
    pub streamer_twitch: &'a api::Twitch,
    pub futures: &'a mut Vec<future::BoxFuture<'static, Result<(), failure::Error>>>,
    pub stream_info: &'a stream_info::StreamInfo,
    pub sender: &'a irc::Sender,
    pub settings: &'a settings::Settings,
    pub idle: &'a idle::Idle,
    pub player: Option<&'a player::Player>,
    pub obs: Option<&'a obs::Obs>,
}

pub trait Module: Send + 'static {
    /// Type of the module as a string to help with diagnostics.
    fn ty(&self) -> &'static str;

    /// Set up command handlers for this module.
    fn hook(&self, _: HookContext<'_>) -> Result<(), failure::Error> {
        Ok(())
    }
}

impl Config {
    pub fn load(&self, config: &config::Config) -> Result<Box<dyn Module>, failure::Error> {
        Ok(match *self {
            Config::Countdown(ref module) => {
                Box::new(self::countdown::Module::load(config, module)?)
            }
            Config::SwearJar(ref module) => Box::new(self::swearjar::Module::load(config, module)?),
            Config::Water(ref module) => Box::new(self::water::Module::load(config, module)?),
            Config::Promotions(ref module) => Box::new(self::promotions::Module::load(module)?),
            Config::Gtav(ref module) => Box::new(self::gtav::Module::load(module)?),
        })
    }
}
