//! Traits and shared plumbing for bot commands (e.g. `!uptime`)

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::num;
use std::str;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use auth::Scope;
use common::Channel;
use common::{display, words, Cooldown};
use tokio::sync;
use tokio::sync::Notify;

use crate::chat::User;
use crate::messages;
use crate::sender;

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
    async fn peek(&self, user: &User, m: &str) -> Result<()>;
}

#[derive(Default)]
pub struct ContextNotify {
    /// Indicate that the bot should be restarted.
    pub restart: Arc<Notify>,
    /// Indicate that moderators should be refreshed.
    pub refresh_mods: Notify,
    /// Indicate that VIPs should be refreshed.
    pub refresh_vips: Notify,
}

impl ContextNotify {
    fn new(restart: Arc<Notify>) -> Self {
        Self {
            restart,
            refresh_mods: Notify::default(),
            refresh_vips: Notify::default(),
        }
    }
}

pub(crate) struct ContextInner {
    /// Sender associated with the command.
    sender: sender::Sender,
    /// Active scope cooldowns.
    scope_cooldowns: sync::Mutex<HashMap<Scope, Cooldown>>,
    /// A hook that can be installed to peek at all incoming messages.
    pub(crate) message_hooks: sync::RwLock<slab::Slab<Box<dyn MessageHook>>>,
    /// Logins for moderators.
    pub(crate) moderators: parking_lot::RwLock<HashSet<String>>,
    /// Logins for VIPs.
    pub(crate) vips: parking_lot::RwLock<HashSet<String>>,
    /// Notifications that can be sent to the context.
    notify: ContextNotify,
}

impl ContextInner {
    pub(crate) fn new(
        sender: sender::Sender,
        scope_cooldowns: HashMap<Scope, Cooldown>,
        restart: Arc<Notify>,
    ) -> Self {
        Self {
            sender,
            scope_cooldowns: sync::Mutex::new(scope_cooldowns),
            message_hooks: Default::default(),
            moderators: Default::default(),
            vips: Default::default(),
            notify: ContextNotify::new(restart),
        }
    }

    /// Available notifications.
    pub(crate) fn notify(&self) -> &ContextNotify {
        &self.notify
    }
}

/// Context for a single command invocation.
#[derive(Clone)]
pub struct Context {
    pub api_url: Arc<Option<String>>,
    pub user: User,
    pub it: words::Split,
    pub messages: messages::Messages,
    pub(crate) inner: Arc<ContextInner>,
}

impl Context {
    /// Get associated sender.
    pub fn sender(&self) -> &sender::Sender {
        &self.inner.sender
    }

    /// Available notifications.
    pub fn notify(&self) -> &ContextNotify {
        &self.inner.notify
    }

    /// Access the last known API url.
    pub fn api_url(&self) -> Option<&str> {
        self.api_url.as_deref()
    }

    /// Get the channel.
    pub fn channel(&self) -> &Channel {
        self.inner.sender.channel()
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
    #[tracing::instrument(skip(self), fields(roles = ?self.user.roles(), principal = ?self.user.principal()))]
    pub async fn check_scope(&self, scope: Scope) -> Result<()> {
        tracing::info!("Checking scope");

        if !self.user.has_scope(scope).await {
            let m = self.messages.get(messages::AUTH_FAILED_RUDE).await;
            self.respond(m).await;
            respond_bail!();
        }

        if self.user.has_scope(Scope::BypassCooldowns).await {
            return Ok(());
        }

        let mut scope_cooldowns = self.inner.scope_cooldowns.lock().await;

        if let Some(cooldown) = scope_cooldowns.get_mut(&scope) {
            let now = Instant::now();

            if let Some(duration) = cooldown.check(now) {
                respond_bail!(
                    "Cooldown in effect for {}",
                    display::compact_duration(duration),
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
            .ok_or(respond_err!("Expected {}", m))?)
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
        Ok(self.next().ok_or(respond_err!("Expected {}", m))?)
    }
}
