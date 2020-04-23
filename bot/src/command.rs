//! Traits and shared plumbing for bot commands (e.g. `!uptime`)

use crate::{auth::Scope, irc, prelude::*, utils};
use anyhow::{bail, Error};
use std::{collections::HashMap, fmt, sync::Arc, time::Instant};
use tokio::sync::Mutex;

#[async_trait]
/// The handler trait for a given command.
pub trait Handler {
    /// Scope required to run command.
    fn scope(&self) -> Option<Scope> {
        None
    }

    /// Handle the command.
    async fn handle(&mut self, ctx: Context) -> Result<(), Error>;
}

#[async_trait]
/// A trait for peeking into chat messages.
pub trait MessageHook: std::any::Any + Send + Sync {
    /// Peek the given message.
    async fn peek(&mut self, user: &irc::User, m: &str) -> Result<(), Error>;
}

pub(crate) struct ContextInner {
    /// Sender associated with the command.
    pub(crate) sender: irc::Sender,
    /// Active scope cooldowns.
    pub(crate) scope_cooldowns: Mutex<HashMap<Scope, utils::Cooldown>>,
    /// A hook that can be installed to peek at all incoming messages.
    pub(crate) message_hooks: Mutex<HashMap<String, Box<dyn MessageHook>>>,
    /// Shutdown handler.
    pub(crate) shutdown: utils::Shutdown,
}

/// Context for a single command invocation.
pub struct Context {
    pub(crate) api_url: Arc<Option<String>>,
    pub(crate) user: irc::User,
    pub(crate) it: utils::Words,
    pub(crate) inner: Arc<ContextInner>,
}

impl Context {
    /// Access the last known API url.
    pub fn api_url(&self) -> Option<&str> {
        self.api_url.as_deref()
    }

    /// Get the channel.
    pub fn channel(&self) -> &str {
        self.inner.sender.channel()
    }

    /// Signal that the bot should try to shut down.
    pub fn shutdown(&self) -> bool {
        self.inner.shutdown.shutdown()
    }

    /// Setup the specified hook.
    pub async fn insert_hook<H>(&self, id: &str, hook: H)
    where
        H: MessageHook,
    {
        let mut hooks = self.inner.message_hooks.lock().await;
        hooks.insert(id.to_string(), Box::new(hook));
    }

    /// Setup the specified hook.
    pub async fn remove_hook(&self, id: &str) {
        let mut hooks = self.inner.message_hooks.lock().await;
        let _ = hooks.remove(id);
    }

    /// Verify that the current user has the associated scope.
    pub async fn check_scope(&self, scope: Scope) -> Result<(), Error> {
        if !self.user.has_scope(scope) {
            if let Some(name) = self.user.display_name() {
                self.privmsg(format!(
                    "Do you think this is a democracy {name}? LUL",
                    name = name,
                ));
            }

            bail!(
                "Scope `{}` not associated with user {:?}",
                scope,
                self.user.name()
            );
        }

        if self.user.has_scope(Scope::BypassCooldowns) {
            return Ok(());
        }

        let mut scope_cooldowns = self.inner.scope_cooldowns.lock().await;

        if let Some(cooldown) = scope_cooldowns.get_mut(&scope) {
            let now = Instant::now();

            if let Some(duration) = cooldown.check(now.clone()) {
                self.respond(format!(
                    "Cooldown in effect for {}",
                    utils::compact_duration(&duration),
                ));

                bail!("Scope `{}` is in cooldown", scope);
            }

            cooldown.poke(now);
        }

        Ok(())
    }

    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        self.user.respond(m);
    }

    /// Send a privmsg to the channel.
    pub fn privmsg(&self, m: impl fmt::Display) {
        self.inner.sender.privmsg(m);
    }

    /// Get the next argument.
    pub fn next(&mut self) -> Option<String> {
        self.it.next()
    }

    /// Get the rest of the commandline.
    pub fn rest(&self) -> &str {
        self.it.rest()
    }

    /// Take the next parameter and parse as the given type.
    pub fn next_parse_optional<T>(&mut self) -> Option<Option<T>>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
    {
        match self.next() {
            Some(s) => match str::parse(&s) {
                Ok(v) => Some(Some(v)),
                Err(e) => {
                    self.respond(format!("Bad argument: {}: {}", s, e));
                    None
                }
            },
            None => Some(None),
        }
    }

    /// Take the next parameter and parse as the given type.
    pub fn next_parse<T, M>(&mut self, m: M) -> Option<T>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
        M: fmt::Display,
    {
        match self.next_parse_optional()? {
            Some(value) => Some(value),
            None => {
                self.respond(format!("Expected {m}", m = m));
                None
            }
        }
    }

    /// Take the rest and parse as the given type.
    pub fn rest_parse<T, M>(&mut self, m: M) -> Option<T>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
        M: fmt::Display,
    {
        match self.rest().trim() {
            "" => {
                self.respond(format!("Expected {m}", m = m));
                None
            }
            s => match str::parse(s) {
                Ok(v) => Some(v),
                Err(e) => {
                    self.respond(format!("Bad argument: {}: {}", s, e));
                    None
                }
            },
        }
    }

    /// Take the next parameter.
    pub fn next_str<M>(&mut self, m: M) -> Option<String>
    where
        M: fmt::Display,
    {
        match self.next() {
            Some(s) => Some(s),
            None => {
                self.respond(format!("Expected {m}", m = m));
                None
            }
        }
    }
}
