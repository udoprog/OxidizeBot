use crate::{api, command, idle, injector, irc, settings, stream_info, utils};
use hashbrown::HashMap;

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

/// Context for a hook.
pub struct HookContext<'a: 'b, 'b> {
    pub injector: &'b injector::Injector,
    pub handlers: &'b mut Handlers<'a>,
    pub futures: &'b mut utils::Futures<'a>,
    pub stream_info: &'b stream_info::StreamInfo,
    pub idle: &'b idle::Idle,
    pub twitch: &'a api::Twitch,
    pub streamer_twitch: &'a api::Twitch,
    pub sender: &'a irc::Sender,
    pub settings: &'a settings::Settings,
    pub auth: &'a crate::auth::Auth,
}

#[async_trait::async_trait]
pub trait Module: 'static + Send + Sync {
    /// Type of the module as a string to help with diagnostics.
    fn ty(&self) -> &'static str;

    /// Set up command handlers for this module.
    async fn hook(&self, _: HookContext<'_, '_>) -> Result<(), failure::Error>;
}
