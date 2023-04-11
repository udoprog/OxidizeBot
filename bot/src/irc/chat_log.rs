use anyhow::Result;

use crate::irc;
use storage::Cache;

pub(crate) struct Builder {
    streamer: api::TwitchAndUser,
    pub(crate) message_log: messagelog::MessageLog,
    pub(crate) cache_stream: async_injector::Stream<Cache>,
    pub(crate) cache: Option<Cache>,
    pub(crate) enabled_stream: settings::Stream<bool>,
    pub(crate) enabled: bool,
    pub(crate) emotes_enabled_stream: settings::Stream<bool>,
    pub(crate) emotes_enabled: bool,
}

impl Builder {
    pub(crate) async fn new(
        streamer: api::TwitchAndUser,
        injector: &async_injector::Injector,
        message_log: messagelog::MessageLog,
        settings: settings::Settings<::auth::Scope>,
    ) -> Result<Self> {
        let (cache_stream, cache) = injector.stream::<Cache>().await;

        let (enabled_stream, enabled) = settings.stream("enabled").or_default().await?;

        let (emotes_enabled_stream, emotes_enabled) =
            settings.stream("emotes-enabled").or_default().await?;

        message_log.enabled(enabled).await;

        Ok(Self {
            streamer,
            message_log,
            cache_stream,
            cache,
            enabled_stream,
            enabled,
            emotes_enabled_stream,
            emotes_enabled,
        })
    }

    /// Update builder.
    pub(crate) async fn update(&mut self) -> Result<Option<ChatLog>> {
        tokio::select! {
            cache = self.cache_stream.recv() => {
                self.cache = cache;
                self.build()
            }
            enabled = self.enabled_stream.recv() => {
                self.enabled = enabled;
                self.message_log.enabled(enabled).await;
                self.build()
            }
            emotes_enabled = self.emotes_enabled_stream.recv() => {
                self.emotes_enabled = emotes_enabled;
                self.build()
            }
        }
    }

    /// Construct a new chat log with the specified configuration.
    pub(crate) fn build(&self) -> Result<Option<ChatLog>> {
        if !self.enabled {
            return Ok(None);
        }

        let emotes = match (self.emotes_enabled, self.cache.as_ref()) {
            (true, Some(cache)) => Some(emotes::Emotes::new(
                crate::USER_AGENT,
                cache.clone(),
                self.streamer.clone(),
            )?),
            _ => None,
        };

        Ok(Some(ChatLog {
            message_log: self.message_log.clone(),
            emotes,
        }))
    }
}

#[derive(Clone)]
pub(crate) struct ChatLog {
    /// Log to add messages to.
    pub(crate) message_log: messagelog::MessageLog,
    /// Handler of emotes.
    emotes: Option<emotes::Emotes>,
}

impl ChatLog {
    pub(crate) async fn observe(
        &self,
        tags: &irc::Tags,
        user: &api::User,
        login: &str,
        message: &str,
    ) {
        let rendered = match self.emotes.as_ref() {
            Some(emotes) => match emotes.render(tags, user, login, message).await {
                Ok(rendered) => Some(rendered),
                Err(e) => {
                    tracing::warn!("Failed to render emotes: {}", e);
                    None
                }
            },
            None => None,
        };

        self.message_log
            .push_back(tags, login, message, rendered)
            .await;
    }
}
