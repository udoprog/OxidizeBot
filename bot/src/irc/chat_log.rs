use crate::api::{self, Twitch};
use crate::emotes;
use crate::injector::{self, Injector};
use crate::irc;
use crate::message_log;
use crate::settings;
use crate::storage::Cache;
use anyhow::Result;

pub struct Builder {
    twitch: Twitch,
    pub(crate) message_log: message_log::MessageLog,
    pub(crate) cache_stream: injector::Stream<Cache>,
    pub(crate) cache: Option<Cache>,
    pub(crate) enabled_stream: settings::Stream<bool>,
    pub(crate) enabled: bool,
    pub(crate) emotes_enabled_stream: settings::Stream<bool>,
    pub(crate) emotes_enabled: bool,
}

impl Builder {
    pub async fn new(
        twitch: Twitch,
        injector: &Injector,
        message_log: message_log::MessageLog,
        settings: crate::Settings,
    ) -> Result<Self> {
        let (cache_stream, cache) = injector.stream::<Cache>().await;

        let (enabled_stream, enabled) = settings.stream("enabled").or_default().await?;

        let (emotes_enabled_stream, emotes_enabled) =
            settings.stream("emotes-enabled").or_default().await?;

        message_log.enabled(enabled).await;

        Ok(Self {
            twitch,
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
    pub async fn update(&mut self) -> Result<Option<ChatLog>> {
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
    pub fn build(&self) -> Result<Option<ChatLog>> {
        if !self.enabled {
            return Ok(None);
        }

        let emotes = match (self.emotes_enabled, self.cache.as_ref()) {
            (true, Some(cache)) => Some(emotes::Emotes::new(cache.clone(), self.twitch.clone())?),
            _ => None,
        };

        Ok(Some(ChatLog {
            message_log: self.message_log.clone(),
            emotes,
        }))
    }
}

#[derive(Clone)]
pub struct ChatLog {
    /// Log to add messages to.
    pub message_log: message_log::MessageLog,
    /// Handler of emotes.
    emotes: Option<emotes::Emotes>,
}

impl ChatLog {
    pub async fn observe(&self, tags: &irc::Tags, user: &api::User, name: &str, message: &str) {
        let rendered = match self.emotes.as_ref() {
            Some(emotes) => match emotes.render(&tags, user, name, message).await {
                Ok(rendered) => Some(rendered),
                Err(e) => {
                    log::warn!("failed to render emotes: {}", e);
                    None
                }
            },
            None => None,
        };

        self.message_log
            .push_back(&tags, &name, message, rendered)
            .await;
    }
}
