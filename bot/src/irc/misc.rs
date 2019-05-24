//! module for misc smaller commands.

use crate::{api, command, irc, stream_info, utils};
use chrono::Utc;

/// Handler for the `!uptime` command.
pub struct Uptime {
    pub stream_info: stream_info::StreamInfo,
}

impl command::Handler for Uptime {
    fn handle<'m>(&mut self, ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        let started_at = self
            .stream_info
            .data
            .read()
            .stream
            .as_ref()
            .map(|s| s.started_at.clone());

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
    pub stream_info: stream_info::StreamInfo,
    pub twitch: api::Twitch,
}

impl Title {
    /// Handle the title command.
    fn show(&mut self, user: irc::User<'_>) {
        let title = self.stream_info.data.read().title.clone();

        match title {
            Some(title) => {
                user.respond(title);
            }
            None => {
                user.respond("Stream is not live right now, try again later!");
            }
        }
    }
}

impl command::Handler for Title {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        let rest = ctx.rest();

        if rest.is_empty() {
            self.show(ctx.user);
        } else {
            ctx.check_moderator()?;

            let twitch = self.twitch.clone();
            let user = ctx.user.as_owned_user();
            let title = rest.to_string();

            ctx.spawn(async move {
                let channel_id = user.target.trim_start_matches('#').to_string();
                let mut request = api::twitch::UpdateChannelRequest::default();
                request.channel.status = Some(title);

                match twitch.update_channel(channel_id.as_str(), request).await {
                    Ok(()) => {
                        user.respond("Title updated!");
                    }
                    Err(e) => {
                        log_err!(e, "failed to update title");
                    }
                }
            });
        }

        Ok(())
    }
}

/// Handler for the `!title` command.
pub struct Game {
    pub stream_info: stream_info::StreamInfo,
    pub twitch: api::Twitch,
}

impl Game {
    /// Handle the game command.
    fn show(&mut self, user: irc::User<'_>) {
        let game = self.stream_info.data.read().game.clone();

        match game {
            Some(game) => {
                user.respond(game);
            }
            None => {
                user.respond("Unfortunately I don't know the game, sorry!");
            }
        };
    }
}

impl command::Handler for Game {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        let rest = ctx.rest();

        if rest.is_empty() {
            self.show(ctx.user);
            return Ok(());
        }

        ctx.check_moderator()?;

        let twitch = self.twitch.clone();
        let user = ctx.user.as_owned_user();
        let game = rest.to_string();

        ctx.spawn(async move {
            let channel_id = user.target.trim_start_matches('#').to_string();
            let mut request = api::twitch::UpdateChannelRequest::default();
            request.channel.game = Some(game);

            match twitch.update_channel(channel_id.as_str(), request).await {
                Ok(()) => {
                    user.respond("Game updated!");
                }
                Err(e) => {
                    log_err!(e, "failed to update game");
                }
            }
        });

        Ok(())
    }
}
