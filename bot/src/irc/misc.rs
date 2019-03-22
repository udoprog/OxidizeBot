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
    fn handle<'m>(
        &mut self,
        _: command::Context<'_>,
        user: irc::User<'m>,
        _: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
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
                let uptime = utils::human_time((now - *started_at).num_seconds());

                user.respond(format!(
                    "Stream has been live for {uptime}.",
                    uptime = uptime
                ));
            }
            Some(_) => {
                user.respond("Stream is live, but start time is weird!");
            }
            None => {
                user.respond("Stream is not live right now, try again later!");
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
    fn show(&mut self, user: &irc::User<'_>) {
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
    fn update(&mut self, user: irc::User<'_>, title: &str) -> impl Future<Item = (), Error = ()> {
        let channel_id = user.target.trim_start_matches('#');

        let twitch = self.twitch.clone();
        let user = user.as_owned_user();
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
    fn handle<'m>(
        &mut self,
        mut ctx: command::Context<'_>,
        user: irc::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        let rest = it.rest();

        if rest.is_empty() {
            self.show(&user);
        } else {
            ctx.check_moderator(&user)?;
            ctx.spawn(self.update(user, rest));
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
    fn update(&mut self, user: irc::User<'_>, game: &str) -> impl Future<Item = (), Error = ()> {
        let channel_id = user.target.trim_start_matches('#');

        let twitch = self.twitch.clone();
        let user = user.as_owned_user();
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
    fn handle<'m>(
        &mut self,
        mut ctx: command::Context<'_>,
        user: irc::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        let rest = it.rest();

        if rest.is_empty() {
            self.show(user);
        } else {
            ctx.check_moderator(&user)?;
            ctx.spawn(self.update(user, rest));
        }

        Ok(())
    }
}
