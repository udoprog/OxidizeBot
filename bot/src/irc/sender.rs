use std::fmt;
use std::sync::Arc;
use std::time;

use anyhow::Result;
use common::Channel;
use irc::client;
use irc::proto::command::{CapSubCommand, Command};
use irc::proto::message::Message;
use leaky_bucket::RateLimiter;

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, Default)]
pub(crate) enum Type {
    #[serde(rename = "chat")]
    #[default]
    Chat,
    #[serde(rename = "nightbot")]
    NightBot,
}

struct Inner {
    target: String,
    sender: client::Sender,
    limiter: RateLimiter,
    nightbot_limiter: RateLimiter,
    nightbot: async_injector::Ref<api::NightBot>,
}

#[derive(Clone)]
pub(crate) struct Sender {
    ty: settings::Var<Type>,
    inner: Arc<Inner>,
}

impl Sender {
    /// Create a new sender.
    pub(crate) fn new(
        ty: settings::Var<Type>,
        target: String,
        sender: client::Sender,
        nightbot: async_injector::Ref<api::NightBot>,
    ) -> Result<Sender> {
        // limiter to use for IRC chat messages.
        let limiter = RateLimiter::builder()
            .interval(time::Duration::from_secs(1))
            .max(95)
            .build();

        let nightbot_limiter = RateLimiter::builder()
            .max(1)
            .interval(time::Duration::from_secs(5))
            .build();

        Ok(Sender {
            ty,
            inner: Arc::new(Inner {
                target,
                sender,
                limiter,
                nightbot_limiter,
                nightbot,
            }),
        })
    }

    /// Get the channel this sender is associated with.
    pub fn channel(&self) -> &Channel {
        Channel::new(self.inner.target.as_str())
    }

    /// Delete the given message by id.
    #[tracing::instrument(skip_all)]
    pub(crate) fn delete(&self, id: &str) {
        self.privmsg_immediate(format!("/delete {}", id));
    }

    /// Only send to chat, with rate limiting.
    #[tracing::instrument(skip_all)]
    pub(crate) async fn send(&self, m: impl Into<Message>) {
        let m = m.into();

        self.inner.limiter.acquire(1).await;

        if let Err(e) = self.inner.sender.send(m) {
            log_error!(e, "Failed to send message");
        }
    }

    /// Send an immediate message, without taking rate limiting into account.
    #[tracing::instrument(skip_all)]
    pub(crate) fn send_immediate(&self, m: impl Into<Message>) {
        if let Err(e) = self.inner.sender.send(m) {
            log_error!(e, "Failed to send message");
        }
    }

    /// Send a PRIVMSG.
    #[tracing::instrument(skip_all)]
    pub(crate) async fn privmsg(&self, f: impl fmt::Display) {
        let message = f.to_string();

        match self.ty.load().await {
            Type::NightBot => {
                tracing::trace!(?message, "Sending to nightbot");
                self.send_nightbot(&self.inner, message).await;
            }
            Type::Chat => {
                tracing::trace!(?message, target = ?self.inner.target, "Sending privmsg");
                self.send(Command::PRIVMSG(self.inner.target.clone(), message))
                    .await;
            }
        }
    }

    /// Send a PRIVMSG without rate limiting.
    pub(crate) fn privmsg_immediate(&self, f: impl fmt::Display) {
        self.send_immediate(Command::PRIVMSG(self.inner.target.clone(), f.to_string()))
    }

    /// Send a capability request.
    pub(crate) async fn cap_req(&self, cap: &str) {
        self.send(Command::CAP(
            None,
            CapSubCommand::REQ,
            Some(String::from(cap)),
            None,
        ))
        .await;
    }

    /// Send message via nightbot.
    async fn send_nightbot(&self, inner: &Inner, m: String) {
        let nightbot = match inner.nightbot.load().await {
            Some(nightbot) => nightbot,
            None => {
                tracing::warn!("Nightbot API is not configured");
                return;
            }
        };

        // wait for the initial permit, keep the lock in case message is rejected.
        inner.nightbot_limiter.acquire(1).await;

        if let Err(e) = nightbot.channel_send(m.clone()).await {
            log_error!(e, "Failed to send message via nightbot");
        }
    }
}
