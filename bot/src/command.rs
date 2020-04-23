//! Traits and shared plumbing for bot commands (e.g. `!uptime`)

use crate::{auth::Scope, irc, prelude::*, utils};
use anyhow::{bail, Error};
use std::{collections::HashMap, fmt, time::Instant};

#[async_trait]
/// The handler trait for a given command.
pub trait Handler {
    /// Scope required to run command.
    fn scope(&self) -> Option<Scope> {
        None
    }

    /// Handle the command.
    async fn handle(&mut self, ctx: Context<'_>) -> Result<(), Error>;
}

/// A trait for peeking into chat messages.
pub trait MessageHook: std::any::Any + Send + Sync {
    /// Peek the given message.
    fn peek(&mut self, user: &irc::User, m: &str) -> Result<(), Error>;
}

/// Context for a single command invocation.
pub struct Context<'a> {
    pub(crate) api_url: Option<&'a str>,
    /// Sender associated with the command.
    pub(crate) sender: &'a irc::Sender,
    pub(crate) user: irc::User,
    pub(crate) it: utils::Words<'a>,
    pub(crate) shutdown: &'a utils::Shutdown,
    pub(crate) scope_cooldowns: &'a mut HashMap<Scope, utils::Cooldown>,
    pub(crate) message_hooks: &'a mut HashMap<String, Box<dyn MessageHook>>,
    pub(crate) respond_buffer: &'a mut String,
}

impl<'a> Context<'a> {
    /// Get the channel.
    pub fn channel(&self) -> &str {
        self.sender.channel()
    }

    /// Verify that the current user has the associated scope.
    pub fn check_scope(&mut self, scope: Scope) -> Result<(), Error> {
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

        if let Some(cooldown) = self.scope_cooldowns.get_mut(&scope) {
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
        self.sender.privmsg(m);
    }

    /// Get the next argument.
    pub fn next(&mut self) -> Option<String> {
        self.it.next()
    }

    /// Get the rest of the commandline.
    pub fn rest(&self) -> &'a str {
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
