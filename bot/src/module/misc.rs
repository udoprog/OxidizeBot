//! module for misc smaller commands.

use std::pin::pin;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use common::display;
use common::stream::StreamExt;

use crate::command;
use crate::irc;
use crate::module;
use crate::stream_info;

/// Handler for the `!uptime` command.
pub(crate) struct Uptime {
    pub(crate) enabled: settings::Var<bool>,
    pub(crate) stream_info: stream_info::StreamInfo,
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
                    display::compact_duration((now - *started_at).to_std().unwrap_or_default());

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
pub(crate) struct Title {
    pub(crate) enabled: settings::Var<bool>,
    pub(crate) stream_info: stream_info::StreamInfo,
    pub(crate) streamer: api::TwitchAndUser,
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

            let mut request = api::twitch::model::ModifyChannelRequest::default();
            request.title = Some(rest);

            self.streamer
                .client
                .patch_channel(&self.streamer.user.id, request)
                .await?;

            self.stream_info.refresh_channel(&self.streamer).await?;
            respond!(ctx, "Title updated!");
        }

        Ok(())
    }
}

/// Handler for the `!game` command.
pub(crate) struct Game {
    pub(crate) enabled: settings::Var<bool>,
    pub(crate) stream_info: stream_info::StreamInfo,
    pub(crate) streamer: api::TwitchAndUser,
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

        let mut stream = pin!(self.streamer.client.categories(rest));

        let first = if let Some(first) = stream.next().await {
            first?
        } else {
            respond!(ctx, "No category found matching `{}`", rest);
            return Ok(());
        };

        let mut request = api::twitch::model::ModifyChannelRequest::default();
        request.game_id = Some(&first.id);

        self.streamer
            .client
            .patch_channel(&self.streamer.user.id, request)
            .await?;

        stream_info.refresh_channel(&self.streamer).await?;

        respond!(ctx, "Game updated to `{}`!", first.name);
        Ok(())
    }
}

pub(crate) struct Module;

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
            streamer,
            settings,
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        handlers.insert(
            "title",
            Title {
                enabled: settings.var("title/enabled", true).await?,
                stream_info: stream_info.clone(),
                streamer: streamer.clone(),
            },
        );

        handlers.insert(
            "game",
            Game {
                enabled: settings.var("game/enabled", true).await?,
                stream_info: stream_info.clone(),
                streamer: streamer.clone(),
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
