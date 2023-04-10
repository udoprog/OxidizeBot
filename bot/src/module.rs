use crate::command;
use crate::idle;
use crate::irc;
use crate::stream_info;

use async_injector::Injector;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;

#[macro_use]
mod macros;
pub(crate) mod admin;
pub(crate) mod after_stream;
pub(crate) mod alias_admin;
pub(crate) mod auth;
pub(crate) mod clip;
pub(crate) mod command_admin;
pub(crate) mod countdown;
pub(crate) mod eight_ball;
pub(crate) mod gtav;
pub(crate) mod help;
pub(crate) mod misc;
pub(crate) mod poll;
pub(crate) mod promotions;
pub(crate) mod song;
pub(crate) mod speedrun;
pub(crate) mod swearjar;
pub(crate) mod theme_admin;
pub(crate) mod time;
pub(crate) mod water;
pub(crate) mod weather;

#[derive(Default)]
pub(crate) struct Handlers {
    handlers: HashMap<String, Arc<dyn command::Handler>>,
}

impl Handlers {
    /// Insert the given handler.
    pub(crate) fn insert(&mut self, command: impl AsRef<str>, handler: impl command::Handler) {
        self.handlers
            .insert(command.as_ref().to_string(), Arc::new(handler));
    }

    /// Lookup the given command mutably.
    pub(crate) fn get(&self, command: &str) -> Option<Arc<dyn command::Handler>> {
        self.handlers.get(command).cloned()
    }
}

/// Context for a hook.
pub(crate) struct HookContext<'a> {
    pub(crate) injector: &'a Injector,
    pub(crate) handlers: &'a mut Handlers,
    pub(crate) futures: &'a mut common::Futures<'static, Result<()>>,
    pub(crate) stream_info: &'a stream_info::StreamInfo,
    pub(crate) idle: &'a idle::Idle,
    // pub(crate) bot: &'a api::TwitchAndUser,
    pub(crate) streamer: &'a api::TwitchAndUser,
    pub(crate) sender: &'a irc::Sender,
    pub(crate) settings: &'a crate::Settings,
}

#[async_trait::async_trait]
pub(crate) trait Module
where
    Self: 'static + Send + Sync,
{
    /// Type of the module as a string to help with diagnostics.
    fn ty(&self) -> &'static str;

    /// Set up command handlers for this module.
    async fn hook(&self, _: HookContext<'_>) -> Result<()>;
}
