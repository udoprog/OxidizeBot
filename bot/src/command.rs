//! Traits and shared plumbing for bot commands (e.g. `!uptime`)

use crate::{auth::Scope, irc, prelude::*, utils};
use failure::Error;
use hashbrown::HashMap;
use std::{fmt, time::Instant};
use tokio_threadpool::ThreadPool;

/// The handler trait for a given command.
pub trait Handler {
    /// Scope required to run command.
    fn scope(&self) -> Option<Scope> {
        None
    }

    /// Handle the command.
    fn handle(&mut self, ctx: &mut Context<'_, '_>) -> Result<(), Error>;
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
    /// Sender associated with the command.
    pub sender: &'a irc::Sender,
    pub moderator_cooldown: Option<&'a mut utils::Cooldown>,
    pub thread_pool: &'a ThreadPool,
    pub user: irc::User<'m>,
    pub it: &'a mut utils::Words<'m>,
    pub shutdown: &'a utils::Shutdown,
    pub alias: Alias<'a>,
    pub scope_cooldowns: &'a mut HashMap<Scope, utils::Cooldown>,
}

impl<'a, 'm> Context<'a, 'm> {
    /// Spawn the given result and log on errors.
    pub fn spawn_result<F>(&self, id: &'static str, future: F)
    where
        F: std::future::Future<Output = Result<(), Error>> + Send + 'static,
    {
        self.spawn(async move {
            if let Err(e) = future.await {
                log::error!("{}: failed: {}", id, e);
            }
        })
    }

    /// Spawn the given future on the thread pool associated with the context.
    pub fn spawn<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.thread_pool
            .spawn(Compat::new(Box::pin(future.unit_error())));
    }

    /// Verify that the current user has the associated scope.
    pub fn check_scope(&mut self, scope: Scope) -> Result<(), Error> {
        if !self.user.has_scope(scope) {
            self.privmsg(format!(
                "Do you think this is a democracy {name}? LUL",
                name = self.user.name
            ));

            failure::bail!(
                "Scope `{}` not associated with user `{}`",
                scope,
                self.user.name
            );
        }

        if let Some(cooldown) = self.scope_cooldowns.get_mut(&scope) {
            let now = Instant::now();

            if let Some(duration) = cooldown.check(now.clone()) {
                self.respond(format!(
                    "Cooldown in effect for {}",
                    utils::compact_duration(&duration),
                ));

                failure::bail!("Scope `{}` is in cooldown", scope);
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
