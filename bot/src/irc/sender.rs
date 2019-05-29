use crate::{api, prelude::*};
use irc::{
    client::{Client, IrcClient},
    proto::{
        command::{CapSubCommand, Command},
        message::Message,
    },
};
use parking_lot::{Mutex, RwLock};
use std::{fmt, sync::Arc, time};
use tokio_threadpool::ThreadPool;

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
    target: Arc<String>,
    client: IrcClient,
    thread_pool: ThreadPool,
    limiter: Mutex<ratelimit::Limiter>,
    nightbot_limiter: Mutex<ratelimit::Limiter>,
    nightbot: Arc<api::NightBot>,
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
        target: Arc<String>,
        client: IrcClient,
        nightbot: Arc<api::NightBot>,
    ) -> Sender {
        // limiter to use for IRC chat messages.
        let limiter = ratelimit::Builder::new().frequency(10).capacity(95).build();
        let limiter = Mutex::new(limiter);

        let nightbot_limiter = ratelimit::Builder::new()
            .quantum(1)
            .capacity(1)
            .interval(time::Duration::from_secs(5))
            .build();
        let nightbot_limiter = Mutex::new(nightbot_limiter);

        Sender {
            ty,
            inner: Arc::new(Inner {
                target,
                client,
                thread_pool: ThreadPool::new(),
                limiter,
                nightbot_limiter,
                nightbot,
            }),
        }
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
        let inner = self.inner.clone();
        let m = m.into();

        self.inner.thread_pool.spawn(future01::lazy(move || {
            inner.limiter.lock().wait();

            if let Err(e) = inner.client.send(m) {
                log_err!(e, "failed to send message");
            }

            Ok(())
        }));
    }

    /// Send an immediate message, without taking rate limiting into account.
    pub fn send_immediate(&self, m: impl Into<Message>) {
        if let Err(e) = self.inner.client.send(m) {
            log_err!(e, "failed to send message");
        }
    }

    /// Send a PRIVMSG.
    pub fn privmsg(&self, f: impl fmt::Display) {
        match *self.ty.read() {
            Type::NightBot => {
                let inner = self.inner.clone();
                self.send_nightbot(inner, f.to_string());
                return;
            }
            Type::Chat => {
                self.send(Command::PRIVMSG(
                    (*self.inner.target).clone(),
                    f.to_string(),
                ));
            }
        }
    }

    /// Send a PRIVMSG without rate limiting.
    pub fn privmsg_immediate(&self, f: impl fmt::Display) {
        self.send_immediate(Command::PRIVMSG(
            (*self.inner.target).clone(),
            f.to_string(),
        ))
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
    fn send_nightbot(&self, inner: Arc<Inner>, m: String) {
        use futures::executor;
        use std::thread;

        let m = m.to_string();

        let future = future01::lazy(move || {
            let mut limiter = inner.nightbot_limiter.lock();
            // wait for the initial permit, keep the lock in case message is rejected.
            limiter.wait();

            loop {
                let result = executor::block_on(inner.nightbot.channel_send(m.clone()));

                match result {
                    Ok(()) => (),
                    Err(api::nightbot::RequestError::TooManyRequests) => {
                        // since we still hold the lock, no one else can send.
                        // sleep for 100 ms an retry the send.
                        thread::sleep(time::Duration::from_millis(1000));
                        continue;
                    }
                    Err(api::nightbot::RequestError::Other(e)) => {
                        log_err!(e, "failed to send message via nightbot");
                    }
                }

                return Ok(());
            }
        });

        self.inner.thread_pool.spawn(future);
    }
}
