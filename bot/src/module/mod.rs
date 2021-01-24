use crate::api;
use crate::command;
use crate::idle;
use crate::injector::Injector;
use crate::irc;
use crate::stream_info;
use crate::utils;
use std::collections::HashMap;
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
pub mod help;
pub mod misc;
pub mod poll;
pub mod promotions;
pub mod song;
pub mod speedrun;
pub mod swearjar;
pub mod theme_admin;
pub mod time;
pub mod water;
pub mod weather;

#[derive(Default)]
pub struct Handlers {
    handlers: HashMap<String, Arc<dyn command::Handler>>,
}

impl Handlers {
    /// Insert the given handler.
    pub fn insert(&mut self, command: impl AsRef<str>, handler: impl command::Handler) {
        self.handlers
            .insert(command.as_ref().to_string(), Arc::new(handler));
    }

    /// Lookup the given command mutably.
    pub fn get(&self, command: &str) -> Option<Arc<dyn command::Handler>> {
        self.handlers.get(command).cloned()
    }
}

/// Context for a hook.
pub struct HookContext<'a> {
    pub injector: &'a Injector,
    pub handlers: &'a mut Handlers,
    pub futures: &'a mut utils::Futures<'static>,
    pub stream_info: &'a stream_info::StreamInfo,
    pub idle: &'a idle::Idle,
    pub twitch: &'a api::Twitch,
    pub streamer_twitch: &'a api::Twitch,
    pub sender: &'a irc::Sender,
    pub settings: &'a crate::Settings,
}

#[async_trait::async_trait]
pub trait Module
where
    Self: 'static + Send + Sync,
{
    /// Type of the module as a string to help with diagnostics.
    fn ty(&self) -> &'static str;

    /// Set up command handlers for this module.
    async fn hook(&self, _: HookContext<'_>) -> Result<(), anyhow::Error>;
}
