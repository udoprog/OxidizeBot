//! module for misc smaller commands.

use crate::{api, auth, command, irc, module, prelude::*, stream_info, utils};
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the `!uptime` command.
pub struct Uptime {
    pub enabled: Arc<RwLock<bool>>,
    pub stream_info: stream_info::StreamInfo,
}

impl command::Handler for Uptime {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Uptime)
    }

    fn handle(&mut self, ctx: &mut command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

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
                    utils::compact_duration(&(now - *started_at).to_std().unwrap_or_default());

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
pub struct Title<'a> {
    pub enabled: Arc<RwLock<bool>>,
    pub stream_info: stream_info::StreamInfo,
    pub twitch: &'a api::Twitch,
}

impl Title<'_> {
    /// Handle the title command.
    fn show(&mut self, user: &irc::User<'_>) {
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

impl command::Handler for Title<'_> {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Title)
    }

    fn handle(&mut self, ctx: &mut command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let rest = ctx.rest();

        if rest.is_empty() {
            self.show(&ctx.user);
        } else {
            ctx.check_scope(auth::Scope::TitleEdit)?;

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
pub struct Game<'a> {
    pub enabled: Arc<RwLock<bool>>,
    pub stream_info: stream_info::StreamInfo,
    pub twitch: &'a api::Twitch,
}

impl Game<'_> {
    /// Handle the game command.
    fn show(&mut self, user: &irc::User<'_>) {
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

impl command::Handler for Game<'_> {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Game)
    }

    fn handle(&mut self, ctx: &mut command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let rest = ctx.rest();

        if rest.is_empty() {
            self.show(&ctx.user);
            return Ok(());
        }

        ctx.check_scope(auth::Scope::GameEdit)?;

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

pub struct Module;

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "misc"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            stream_info,
            streamer_twitch,
            settings,
            futures,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        let mut vars = settings.vars();

        handlers.insert(
            "title",
            Title {
                enabled: vars.var("title/enabled", true)?,
                stream_info: stream_info.clone(),
                twitch: &streamer_twitch,
            },
        );

        handlers.insert(
            "game",
            Game {
                enabled: vars.var("game/enabled", true)?,
                stream_info: stream_info.clone(),
                twitch: &streamer_twitch,
            },
        );

        handlers.insert(
            "uptime",
            Uptime {
                enabled: vars.var("uptime/enabled", true)?,
                stream_info: stream_info.clone(),
            },
        );

        futures.push(vars.run().boxed());
        Ok(())
    }
}
