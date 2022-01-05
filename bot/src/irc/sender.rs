use crate::api;
use crate::injector;
use crate::settings;
use anyhow::Result;
use irc::client;
use irc::proto::command::{CapSubCommand, Command};
use irc::proto::message::Message;
use leaky_bucket::RateLimiter;
use std::fmt;
use std::sync::Arc;
use std::time;

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Type {
    #[serde(rename = "chat")]
    Chat,
    #[serde(rename = "nightbot")]
    NightBot,
}

impl Default for Type {
    fn default() -> Self {
        Type::Chat
    }
}

struct Inner {
    target: String,
    sender: client::Sender,
    limiter: RateLimiter,
    nightbot_limiter: RateLimiter,
    nightbot: injector::Ref<api::NightBot>,
}

#[derive(Clone)]
pub struct Sender {
    ty: settings::Var<Type>,
    inner: Arc<Inner>,
}

impl Sender {
    /// Create a new sender.
    pub fn new(
        ty: settings::Var<Type>,
        target: String,
        sender: client::Sender,
        nightbot: injector::Ref<api::NightBot>,
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
    pub fn channel(&self) -> &str {
        self.inner.target.as_str()
    }

    /// Delete the given message by id.
    pub fn delete(&self, id: &str) {
        self.privmsg_immediate(format!("/delete {}", id));
    }

    /// Get list of mods.
    pub fn mods(&self) {
        self.privmsg_immediate("/mods");
    }

    /// Get list of VIPs.
    pub fn vips(&self) {
        self.privmsg_immediate("/vips");
    }

    /// Only send to chat, with rate limiting.
    pub async fn send(&self, m: impl Into<Message>) {
        let m = m.into();

        self.inner.limiter.acquire(1).await;

        if let Err(e) = self.inner.sender.send(m) {
            log_error!(e, "failed to send message");
        }
    }

    /// Send an immediate message, without taking rate limiting into account.
    pub fn send_immediate(&self, m: impl Into<Message>) {
        if let Err(e) = self.inner.sender.send(m) {
            log_error!(e, "failed to send message");
        }
    }

    /// Send a PRIVMSG.
    pub async fn privmsg(&self, f: impl fmt::Display) {
        match self.ty.load().await {
            Type::NightBot => {
                self.send_nightbot(&*self.inner, f.to_string()).await;
            }
            Type::Chat => {
                self.send(Command::PRIVMSG(self.inner.target.clone(), f.to_string()))
                    .await;
            }
        }
    }

    /// Send a PRIVMSG without rate limiting.
    pub fn privmsg_immediate(&self, f: impl fmt::Display) {
        self.send_immediate(Command::PRIVMSG(self.inner.target.clone(), f.to_string()))
    }

    /// Send a capability request.
    pub async fn cap_req(&self, cap: &str) {
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
                log::warn!("Nightbot API is not configured");
                return;
            }
        };

        // wait for the initial permit, keep the lock in case message is rejected.
        inner.nightbot_limiter.acquire(1).await;

        loop {
            let result = nightbot.channel_send(m.clone()).await;

            match result {
                Ok(()) => (),
                Err(api::nightbot::RequestError::TooManyRequests) => {
                    // since we still hold the lock, no one else can send.
                    // sleep for 100 ms an retry the send.
                    tokio::time::sleep(time::Duration::from_millis(1000)).await;

                    continue;
                }
                Err(api::nightbot::RequestError::Other(e)) => {
                    log_error!(e, "failed to send message via nightbot");
                }
            }

            break;
        }
    }
}
