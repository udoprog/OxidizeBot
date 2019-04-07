use crate::{command, config, currency, db, idle, irc, settings, stream_info, twitch, utils};
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_core::reactor::Core;

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

pub mod alias_admin;
pub mod command_admin;
mod countdown;
mod promotions;
pub mod song;
mod swearjar;
mod water;

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
    pub core: &'a mut Core,
    pub config: &'a config::Config,
    pub irc_config: &'a irc::Config,
    pub db: &'a db::Database,
    pub commands: &'a db::Commands,
    pub aliases: &'a db::Aliases,
    pub promotions: &'a db::Promotions,
    pub handlers: &'a mut Handlers,
    pub currency: Option<&'a currency::Currency>,
    pub twitch: &'a twitch::Twitch,
    pub futures: &'a mut Vec<utils::BoxFuture<(), failure::Error>>,
    pub stream_info: &'a Arc<RwLock<stream_info::StreamInfo>>,
    pub sender: &'a irc::Sender,
    pub settings: &'a settings::Settings,
    pub idle: &'a idle::Idle,
}

pub trait Module: 'static {
    /// Get the type of the module as a string.
    fn ty(&self) -> &'static str {
        "unknown"
    }

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
        })
    }
}
