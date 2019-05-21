//! Traits and shared plumbing for bot commands (e.g. `!uptime`)

use crate::{irc, prelude::*, utils};
use hashbrown::HashSet;
use std::fmt;
use tokio_threadpool::ThreadPool;

/// The handler trait for a given command.
pub trait Handler {
    /// Handle the command.
    fn handle<'m>(&mut self, ctx: Context<'_, '_>) -> Result<(), failure::Error>;
}

/// The alias that was expanded for this command.
pub struct Alias<'a> {
    pub alias: Option<(&'a str, &'a str)>,
}

impl Alias<'_> {
    /// Unwrap the given alias, or decode it.
    pub fn unwrap_or(&self, default: &str) -> String {
        let (alias, expanded) = match self.alias {
            Some((alias, expanded)) => (alias, expanded),
            None => return default.to_string(),
        };

        let mut out = Vec::new();
        out.push(alias.to_string());

        let skip = utils::Words::new(expanded).count();

        out.extend(utils::Words::new(default).skip(skip).map(|s| s.to_string()));

        out.join(" ")
    }
}

/// Context for a single command invocation.
pub struct Context<'a, 'm> {
    pub api_url: Option<&'a str>,
    /// The current streamer.
    pub streamer: &'a str,
    /// Sender associated with the command.
    pub sender: &'a irc::Sender,
    /// Moderators.
    pub moderators: &'a HashSet<String>,
    pub moderator_cooldown: Option<&'a mut utils::Cooldown>,
    pub thread_pool: &'a ThreadPool,
    pub user: irc::User<'m>,
    pub it: &'a mut utils::Words<'m>,
    pub shutdown: &'a utils::Shutdown,
    pub alias: Alias<'a>,
}

impl<'a, 'm> Context<'a, 'm> {
    /// Spawn the given future on the thread pool associated with the context.
    pub fn spawn_async<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.thread_pool
            .spawn(Compat::new(Box::pin(future.unit_error())));
    }

    /// Test if streamer.
    pub fn is_streamer(&self) -> bool {
        self.user.name == self.streamer
    }

    /// Test if moderator.
    pub fn is_moderator(&self) -> bool {
        self.moderators.contains(self.user.name) || self.is_streamer()
    }

    /// Check that the given user is a moderator.
    pub fn check_moderator(&mut self) -> Result<(), failure::Error> {
        // Streamer immune to cooldown and is always a moderator.
        if self.user.name == self.streamer {
            return Ok(());
        }

        if !self.is_moderator() {
            self.privmsg(format!(
                "Do you think this is a democracy {name}? LUL",
                name = self.user.name
            ));

            failure::bail!("moderator access required for action");
        }

        // Test if we have moderator cooldown in effect.
        let moderator_cooldown = match self.moderator_cooldown.as_mut() {
            Some(moderator_cooldown) => moderator_cooldown,
            None => return Ok(()),
        };

        if moderator_cooldown.is_open() {
            return Ok(());
        }

        self.privmsg(format!(
            "{name} -> Cooldown in effect since last moderator action.",
            name = self.user.name
        ));

        failure::bail!("moderator action cooldown");
    }

    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        self.user.respond(m);
    }

    /// Send a privmsg to the channel.
    pub fn privmsg(&self, m: impl fmt::Display) {
        self.sender.privmsg(self.user.target, m);
    }

    /// Get the next argument.
    pub fn next(&mut self) -> Option<&'m str> {
        self.it.next()
    }

    /// Get the rest of the commandline.
    pub fn rest(&self) -> &'m str {
        self.it.rest()
    }

    /// Take the next parameter and parse as the given type.
    pub fn next_parse_optional<T>(&mut self) -> Option<Option<T>>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
    {
        match self.next() {
            Some(s) => match str::parse(s) {
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
    pub fn next_parse<T, M>(&mut self, m: M, p: &str) -> Option<T>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
        M: fmt::Display,
    {
        match self.next_parse_optional()? {
            Some(value) => Some(value),
            None => {
                self.respond(format!(
                    "Expected: {p} {m}",
                    p = self.alias.unwrap_or(p),
                    m = m
                ));
                None
            }
        }
    }

    /// Take the rest and parse as the given type.
    pub fn rest_parse<T, M>(&mut self, m: M, p: &str) -> Option<T>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
        M: fmt::Display,
    {
        match self.rest().trim() {
            "" => {
                self.respond(format!(
                    "Expected: {p} {m}",
                    p = self.alias.unwrap_or(p),
                    m = m
                ));
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
    pub fn next_str<M>(&mut self, m: M, p: &str) -> Option<&'m str>
    where
        M: fmt::Display,
    {
        match self.next() {
            Some(s) => Some(s),
            None => {
                self.respond(format!(
                    "Expected: {p} {m}",
                    p = self.alias.unwrap_or(p),
                    m = m
                ));
                None
            }
        }
    }
}
