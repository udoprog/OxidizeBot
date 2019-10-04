use crate::api;
use failure::Error;
use irc::{
    client,
    proto::{
        command::{CapSubCommand, Command},
        message::Message,
    },
};
use leaky_bucket::{LeakyBucket, LeakyBuckets};
use parking_lot::RwLock;
use std::{fmt, sync::Arc, time};

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
    limiter: LeakyBucket,
    nightbot_limiter: LeakyBucket,
    nightbot: Arc<RwLock<Option<Arc<api::NightBot>>>>,
}

#[derive(Clone)]
pub struct Sender {
    ty: Arc<RwLock<Type>>,
    inner: Arc<Inner>,
}

impl Sender {
    /// Create a new sender.
    pub fn new(
        ty: Arc<RwLock<Type>>,
        target: String,
        sender: client::Sender,
        nightbot: Arc<RwLock<Option<Arc<api::NightBot>>>>,
        buckets: &LeakyBuckets,
    ) -> Result<Sender, Error> {
        // limiter to use for IRC chat messages.
        let limiter = buckets
            .rate_limiter()
            .refill_amount(10)
            .refill_interval(time::Duration::from_secs(1))
            .max(95)
            .build()?;

        let nightbot_limiter = buckets
            .rate_limiter()
            .max(1)
            .refill_interval(time::Duration::from_secs(5))
            .build()?;

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
    pub fn send(&self, m: impl Into<Message>) {
        let m = m.into();

        let inner = self.inner.clone();

        tokio::spawn(async move {
            if let Err(e) = inner.limiter.acquire(1).await {
                log_err!(e, "error in limiter");
                return;
            }

            if let Err(e) = inner.sender.send(m) {
                log_err!(e, "failed to send message");
            }
        });
    }

    /// Send an immediate message, without taking rate limiting into account.
    pub fn send_immediate(&self, m: impl Into<Message>) {
        if let Err(e) = self.inner.sender.send(m) {
            log_err!(e, "failed to send message");
        }
    }

    /// Send a PRIVMSG.
    pub fn privmsg(&self, f: impl fmt::Display) {
        match *self.ty.read() {
            Type::NightBot => {
                self.send_nightbot(&*self.inner, f.to_string());
                return;
            }
            Type::Chat => {
                self.send(Command::PRIVMSG(self.inner.target.clone(), f.to_string()));
            }
        }
    }

    /// Send a PRIVMSG without rate limiting.
    pub fn privmsg_immediate(&self, f: impl fmt::Display) {
        self.send_immediate(Command::PRIVMSG(self.inner.target.clone(), f.to_string()))
    }

    /// Send a capability request.
    pub fn cap_req(&self, cap: &str) {
        self.send(Command::CAP(
            None,
            CapSubCommand::REQ,
            Some(String::from(cap)),
            None,
        ))
    }

    /// Send message via nightbot.
    fn send_nightbot(&self, inner: &Inner, m: String) {
        let nightbot = match inner.nightbot.read().as_ref() {
            Some(nightbot) => nightbot.clone(),
            None => {
                log::warn!("Nightbot API is not configured");
                return;
            }
        };

        let m = m.to_string();
        let limiter = inner.nightbot_limiter.clone();

        let future = async move {
            // wait for the initial permit, keep the lock in case message is rejected.
            if let Err(e) = limiter.acquire(1).await {
                log_err!(e, "error in limiter");
                return;
            }

            loop {
                let result = nightbot.channel_send(m.clone()).await;

                match result {
                    Ok(()) => (),
                    Err(api::nightbot::RequestError::TooManyRequests) => {
                        // since we still hold the lock, no one else can send.
                        // sleep for 100 ms an retry the send.
                        tokio::timer::delay(
                            time::Instant::now() + time::Duration::from_millis(1000),
                        )
                        .await;

                        continue;
                    }
                    Err(api::nightbot::RequestError::Other(e)) => {
                        log_err!(e, "failed to send message via nightbot");
                    }
                }

                break;
            }
        };

        tokio::spawn(future);
    }
}
