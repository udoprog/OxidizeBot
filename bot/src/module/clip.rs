use crate::api;
use crate::auth;
use crate::command;
use crate::module;
use crate::prelude::*;
use crate::utils::{Cooldown, Duration};
use anyhow::Result;

/// Handler for the `!clip` command.
pub(crate) struct Clip {
    enabled: settings::Var<bool>,
    clip_cooldown: settings::Var<Cooldown>,
    streamer: api::TwitchAndUser,
}

#[async_trait]
impl command::Handler for Clip {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Clip)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        if !self.clip_cooldown.write().await.is_open() {
            respond!(ctx, "A clip was already created recently");
            return Ok(());
        }

        let title = match ctx.rest().trim() {
            "" => None,
            other => Some(other.to_string()),
        };

        match self
            .streamer
            .client
            .create_clip(&self.streamer.user.id)
            .await?
        {
            Some(clip) => {
                respond!(
                    ctx,
                    "Created clip at {}/{}",
                    api::twitch::CLIPS_URL,
                    clip.id
                );

                if let Some(_title) = title {
                    tracing::warn!("Title was requested, but it can't be set (right now)")
                }
            }
            None => {
                respond!(ctx, "Failed to create clip, sorry :(");
                tracing::error!("Created clip, but API returned nothing");
            }
        }

        Ok(())
    }
}

pub(crate) struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "clip"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            streamer,
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        let settings = settings.scoped("clip");

        handlers.insert(
            "clip",
            Clip {
                enabled: settings.var("enabled", true).await?,
                clip_cooldown: settings
                    .var("cooldown", Cooldown::from_duration(Duration::seconds(30)))
                    .await?,
                streamer: streamer.clone(),
            },
        );

        Ok(())
    }
}
