//! module for misc smaller commands.

use crate::api;
use crate::auth;
use crate::command;
use crate::irc;
use crate::module;
use crate::prelude::*;
use crate::stream_info;
use crate::utils;
use anyhow::Result;
use chrono::Utc;

/// Handler for the `!uptime` command.
pub struct Uptime {
    pub enabled: settings::Var<bool>,
    pub stream_info: stream_info::StreamInfo,
}

#[async_trait]
impl command::Handler for Uptime {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Uptime)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let started_at = self
            .stream_info
            .data
            .read()
            .stream
            .as_ref()
            .map(|s| s.started_at);

        let now = Utc::now();

        match started_at {
            // NB: very important to check that _now_ is after started at.
            Some(ref started_at) if now > *started_at => {
                let uptime =
                    utils::compact_duration((now - *started_at).to_std().unwrap_or_default());

                respond!(ctx, "Stream has been live for {uptime}.", uptime = uptime);
            }
            Some(_) => {
                respond!(ctx, "Stream is live, but start time is weird!");
            }
            None => {
                respond!(ctx, "Stream is not live right now, try again later!");
            }
        }

        Ok(())
    }
}

/// Handler for the `!title` command.
pub struct Title {
    pub enabled: settings::Var<bool>,
    pub stream_info: stream_info::StreamInfo,
    pub twitch: api::Twitch,
}

impl Title {
    /// Handle the title command.
    async fn show(&self, user: &irc::User) {
        let title = self.stream_info.data.read().title.clone();

        match title {
            Some(title) => {
                user.respond(title).await;
            }
            None => {
                user.respond("Stream is not live right now, try again later!")
                    .await;
            }
        }
    }
}

#[async_trait]
impl command::Handler for Title {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Title)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let rest = ctx.rest();

        if rest.is_empty() {
            self.show(&ctx.user).await;
        } else {
            ctx.check_scope(auth::Scope::TitleEdit).await?;

            let user = ctx.user.clone();

            let mut request = api::twitch::new::ModifyChannelRequest::default();
            request.title = Some(rest);
            self.twitch
                .new_modify_channel(&user.streamer().id, request)
                .await?;
            self.stream_info
                .refresh_channel(&self.twitch, user.streamer())
                .await?;

            respond!(ctx, "Title updated!");
        }

        Ok(())
    }
}

/// Handler for the `!game` command.
pub struct Game {
    pub enabled: settings::Var<bool>,
    pub stream_info: stream_info::StreamInfo,
    pub twitch: api::Twitch,
}

impl Game {
    /// Handle the game command.
    async fn show(&self, user: &irc::User) {
        let game = self.stream_info.data.read().game.clone();

        match game {
            Some(game) => {
                user.respond(game).await;
            }
            None => {
                user.respond("Unfortunately I don't know the game, sorry!")
                    .await;
            }
        };
    }
}

#[async_trait]
impl command::Handler for Game {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Game)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let rest = ctx.rest();

        if rest.is_empty() {
            self.show(&ctx.user).await;
            return Ok(());
        }

        ctx.check_scope(auth::Scope::GameEdit).await?;

        let stream_info = self.stream_info.clone();

        let stream = self.twitch.new_search_categories(rest);
        tokio::pin!(stream);

        let first = if let Some(first) = stream.next().await {
            first?
        } else {
            respond!(ctx, "No category found matching `{}`", rest);
            return Ok(());
        };

        let mut request = api::twitch::new::ModifyChannelRequest::default();
        request.game_id = Some(&first.id);

        self.twitch
            .new_modify_channel(&ctx.user.streamer().id, request)
            .await?;

        stream_info
            .refresh_channel(&self.twitch, ctx.user.streamer())
            .await?;

        respond!(ctx, "Game updated to `{}`!", first.name);
        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "misc"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            stream_info,
            streamer_twitch,
            settings,
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        handlers.insert(
            "title",
            Title {
                enabled: settings.var("title/enabled", true).await?,
                stream_info: stream_info.clone(),
                twitch: streamer_twitch.clone(),
            },
        );

        handlers.insert(
            "game",
            Game {
                enabled: settings.var("game/enabled", true).await?,
                stream_info: stream_info.clone(),
                twitch: streamer_twitch.clone(),
            },
        );

        handlers.insert(
            "uptime",
            Uptime {
                enabled: settings.var("uptime/enabled", true).await?,
                stream_info: stream_info.clone(),
            },
        );

        Ok(())
    }
}
