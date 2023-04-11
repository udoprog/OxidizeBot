use async_injector::Injector;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::chat;
use crate::command;
use crate::idle;
use crate::stream_info;

use anyhow::Result;

#[derive(Default)]
pub struct Handlers {
    handlers: HashMap<String, Arc<dyn command::Handler>>,
}

/// Collection of handlers that can be used in a module.
impl Handlers {
    /// Insert the given handler.
    pub fn insert(&mut self, command: impl AsRef<str>, handler: impl command::Handler) {
        self.handlers
            .insert(command.as_ref().to_string(), Arc::new(handler));
    }

    /// Lookup the given command mutably.
    pub(crate) fn get(&self, command: &str) -> Option<Arc<dyn command::Handler>> {
        self.handlers.get(command).cloned()
    }
}

/// Context for hooking up a module.
pub struct HookContext<'a> {
    pub injector: &'a Injector,
    pub handlers: &'a mut Handlers,
    pub futures: &'a mut common::Futures<'static, Result<()>>,
    pub stream_info: &'a stream_info::StreamInfo,
    pub idle: &'a idle::Idle,
    pub streamer: &'a api::TwitchAndUser,
    pub sender: &'a chat::Sender,
    pub settings: &'a settings::Settings<::auth::Scope>,
}

/// Trait used to hook up a module.
#[async_trait]
pub trait Module
where
    Self: 'static + Send + Sync,
{
    /// Type of the module as a string to help with diagnostics.
    fn ty(&self) -> &'static str;

    /// Set up command handlers for this module.
    async fn hook(&self, _: HookContext<'_>) -> Result<()>;
}
