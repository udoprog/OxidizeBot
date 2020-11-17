//! Traits and shared plumbing for bot commands (e.g. `!uptime`)

use crate::auth::Scope;
use crate::irc;
use crate::prelude::*;
use crate::utils;
use anyhow::Result;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::num;
use std::str;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::sync;

#[derive(Debug, Error)]
#[error("Command failed with: {0}")]
pub struct Respond(pub(crate) Cow<'static, str>);

/// An opaque identifier for a hook that has been inserted.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookId(usize);

impl fmt::Display for HookId {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl str::FromStr for HookId {
    type Err = num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HookId(str::parse::<usize>(s)?))
    }
}

#[async_trait]
/// The handler trait for a given command.
pub trait Handler
where
    Self: 'static + Send + Sync,
{
    /// Scope required to run command.
    fn scope(&self) -> Option<Scope> {
        None
    }

    /// Handle the command.
    async fn handle(&self, ctx: &mut Context) -> Result<()>;
}

#[async_trait]
/// A trait for peeking into chat messages.
pub trait MessageHook: std::any::Any + Send + Sync {
    /// Peek the given message.
    async fn peek(&self, user: &irc::User, m: &str) -> Result<()>;
}

pub(crate) struct ContextInner {
    /// Sender associated with the command.
    pub(crate) sender: irc::Sender,
    /// Active scope cooldowns.
    pub(crate) scope_cooldowns: sync::Mutex<HashMap<Scope, utils::Cooldown>>,
    /// A hook that can be installed to peek at all incoming messages.
    pub(crate) message_hooks: sync::RwLock<slab::Slab<Box<dyn MessageHook>>>,
    /// Shutdown handler.
    pub(crate) restart: utils::Restart,
}

/// Context for a single command invocation.
#[derive(Clone)]
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
    pub async fn restart(&self) -> bool {
        self.inner.restart.restart().await
    }

    /// Setup the specified hook.
    pub async fn insert_hook<H>(&self, hook: H) -> HookId
    where
        H: MessageHook,
    {
        let mut hooks = self.inner.message_hooks.write().await;
        let len = hooks.insert(Box::new(hook));
        HookId(len)
    }

    /// Setup the specified hook.
    pub async fn remove_hook(&self, id: HookId) {
        let mut hooks = self.inner.message_hooks.write().await;

        if hooks.contains(id.0) {
            let _ = hooks.remove(id.0);
        }
    }

    /// Verify that the current user has the associated scope.
    pub async fn check_scope(&self, scope: Scope) -> Result<()> {
        if !self.user.has_scope(scope).await {
            respond_bail!("Do you think this is a democracy? LUL");
        }

        if self.user.has_scope(Scope::BypassCooldowns).await {
            return Ok(());
        }

        let mut scope_cooldowns = self.inner.scope_cooldowns.lock().await;

        if let Some(cooldown) = scope_cooldowns.get_mut(&scope) {
            let now = Instant::now();

            if let Some(duration) = cooldown.check(now.clone()) {
                respond_bail!(
                    "Cooldown in effect for {}",
                    utils::compact_duration(duration),
                )
            }

            cooldown.poke(now);
        }

        Ok(())
    }

    /// Respond to the user with a message.
    pub async fn respond(&self, m: impl fmt::Display) {
        self.user.respond(m).await;
    }

    /// Render an iterable of results, that implements display.
    pub async fn respond_lines<I>(&self, results: I, empty: &str)
    where
        I: IntoIterator,
        I::Item: fmt::Display,
    {
        self.user.respond_lines(results, empty).await
    }

    /// Send a privmsg to the channel.
    pub async fn privmsg(&self, m: impl fmt::Display) {
        self.inner.sender.privmsg(m).await;
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
    pub fn next_parse_optional<T>(&mut self) -> Result<Option<T>>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
    {
        Ok(match self.next() {
            Some(s) => match str::parse(&s) {
                Ok(v) => Some(v),
                Err(e) => {
                    respond_bail!("Bad argument: {}: {}", s, e);
                }
            },
            None => None,
        })
    }

    /// Take the next parameter and parse as the given type.
    pub fn next_parse<T, M>(&mut self, m: M) -> Result<T>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
        M: fmt::Display,
    {
        Ok(self
            .next_parse_optional()?
            .ok_or_else(|| respond_err!("Expected {}", m))?)
    }

    /// Take the rest and parse as the given type.
    pub fn rest_parse<T, M>(&mut self, m: M) -> Result<T>
    where
        T: std::str::FromStr,
        T::Err: fmt::Display,
        M: fmt::Display,
    {
        Ok(match self.rest().trim() {
            "" => {
                respond_bail!("Expected {m}", m = m);
            }
            s => match str::parse(s) {
                Ok(v) => v,
                Err(e) => {
                    respond_bail!("Bad argument: {}: {}", s, e);
                }
            },
        })
    }

    /// Take the next parameter.
    pub fn next_str<M>(&mut self, m: M) -> Result<String>
    where
        M: fmt::Display,
    {
        Ok(self.next().ok_or_else(|| respond_err!("Expected {}", m))?)
    }
}
