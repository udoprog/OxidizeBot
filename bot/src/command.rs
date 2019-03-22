//! Traits and shared plumbing for bot commands (e.g. `!uptime`)

use crate::{irc, utils};
use futures::Future;
use hashbrown::HashSet;
use tokio_threadpool::ThreadPool;

/// The handler trait for a given command.
pub trait Handler {
    /// Handle the command.
    fn handle<'m>(
        &mut self,
        ctx: Context<'_>,
        user: irc::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error>;
}

/// Context for a single command invocation.
pub struct Context<'a> {
    pub api_url: Option<&'a str>,
    /// The current streamer.
    pub streamer: &'a str,
    /// Sender associated with the command.
    pub sender: &'a irc::Sender,
    /// Moderators.
    pub moderators: &'a HashSet<String>,
    pub moderator_cooldown: Option<&'a mut utils::Cooldown>,
    pub thread_pool: &'a ThreadPool,
}

impl<'a> Context<'a> {
    /// Spawn the given future on the thread pool associated with the context.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Item = (), Error = ()> + Send + 'static,
    {
        self.thread_pool.spawn(future);
    }

    /// Test if moderator.
    pub fn is_moderator(&self, user: &irc::User<'_>) -> bool {
        self.moderators.contains(user.name)
    }

    /// Check that the given user is a moderator.
    pub fn check_moderator(&mut self, user: &irc::User) -> Result<(), failure::Error> {
        // Streamer immune to cooldown and is always a moderator.
        if user.name == self.streamer {
            return Ok(());
        }

        if !self.is_moderator(user) {
            self.sender.privmsg(
                &user.target,
                format!(
                    "Do you think this is a democracy {name}? LUL",
                    name = user.name
                ),
            );

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

        self.sender.privmsg(
            &user.target,
            format!(
                "{name} -> Cooldown in effect since last moderator action.",
                name = user.name
            ),
        );

        failure::bail!("moderator action cooldown");
    }
}
