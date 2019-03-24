//! module for misc smaller commands.

use crate::{command, irc, twitch, utils};
use chrono::Utc;
use futures::Future;
use std::sync::{Arc, RwLock};

/// Handler for the `!uptime` command.
pub struct Uptime {
    pub stream_info: Arc<RwLock<Option<irc::StreamInfo>>>,
}

impl command::Handler for Uptime {
    fn handle<'m>(&mut self, ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        let started_at = self
            .stream_info
            .read()
            .expect("poisoned")
            .as_ref()
            .and_then(|s| s.stream.as_ref().map(|s| s.started_at.clone()));

        let now = Utc::now();

        match started_at {
            // NB: very important to check that _now_ is after started at.
            Some(ref started_at) if now > *started_at => {
                let uptime =
                    utils::compact_duration((now - *started_at).to_std().unwrap_or_default());

                ctx.respond(format!(
                    "Stream has been live for {uptime}.",
                    uptime = uptime
                ));
            }
            Some(_) => {
                ctx.respond("Stream is live, but start time is weird!");
            }
            None => {
                ctx.respond("Stream is not live right now, try again later!");
            }
        }

        Ok(())
    }
}

/// Handler for the `!title` command.
pub struct Title {
    pub stream_info: Arc<RwLock<Option<irc::StreamInfo>>>,
    pub twitch: twitch::Twitch,
}

impl Title {
    /// Handle the title command.
    fn show(&mut self, user: irc::User<'_>) {
        let title = self
            .stream_info
            .read()
            .expect("poisoned")
            .as_ref()
            .map(|s| s.title.clone());

        match title {
            Some(title) => {
                user.respond(title);
            }
            None => {
                user.respond("Stream is not live right now, try again later!");
            }
        }
    }

    /// Handle the title update.
    fn update(&mut self, user: irc::OwnedUser, title: &str) -> impl Future<Item = (), Error = ()> {
        let channel_id = user.target.trim_start_matches('#');

        let twitch = self.twitch.clone();
        let title = title.to_string();

        let mut request = twitch::UpdateChannelRequest::default();
        request.channel.status = Some(title);

        twitch
            .update_channel(channel_id, &request)
            .and_then(move |_| {
                user.respond("Title updated!");
                Ok(())
            })
            .or_else(|e| {
                utils::log_err("failed to update title", e);
                Ok(())
            })
    }
}

impl command::Handler for Title {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        let rest = ctx.rest();

        if rest.is_empty() {
            self.show(ctx.user);
        } else {
            ctx.check_moderator()?;
            let future = self.update(ctx.user.as_owned_user(), rest);
            ctx.spawn(future);
        }

        Ok(())
    }
}

/// Handler for the `!title` command.
pub struct Game {
    pub stream_info: Arc<RwLock<Option<irc::StreamInfo>>>,
    pub twitch: twitch::Twitch,
}

impl Game {
    /// Handle the game command.
    fn show(&mut self, user: irc::User<'_>) {
        let game = self
            .stream_info
            .read()
            .expect("poisoned")
            .as_ref()
            .and_then(|s| s.game.clone());

        match game {
            Some(game) => {
                user.respond(game);
            }
            None => {
                user.respond("Unfortunately I don't know the game, sorry!");
            }
        };
    }

    /// Handle the game update.
    fn update(&mut self, user: irc::OwnedUser, game: &str) -> impl Future<Item = (), Error = ()> {
        let channel_id = user.target.trim_start_matches('#');

        let twitch = self.twitch.clone();
        let game = game.to_string();

        let mut request = twitch::UpdateChannelRequest::default();
        request.channel.game = Some(game);

        twitch
            .update_channel(channel_id, &request)
            .and_then(move |_| {
                user.respond("Game updated!");
                Ok(())
            })
            .or_else(|e| {
                utils::log_err("failed to update game", e);
                Ok(())
            })
    }
}

impl command::Handler for Game {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        let rest = ctx.rest();

        if rest.is_empty() {
            self.show(ctx.user);
        } else {
            ctx.check_moderator()?;
            let future = self.update(ctx.user.as_owned_user(), rest);
            ctx.spawn(future);
        }

        Ok(())
    }
}
