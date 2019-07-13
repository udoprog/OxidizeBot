use crate::{
    api::{twitch::Channel, Twitch},
    emotes, injector, irc, message_log, settings,
    storage::Cache,
};
use failure::Error;

pub struct Builder {
    message_log: message_log::MessageLog,
    twitch: Twitch,
    pub cache_stream: injector::Stream<Cache>,
    pub cache: Option<Cache>,
    pub enabled_stream: settings::Stream<bool>,
    enabled: bool,
    pub emotes_enabled_stream: settings::Stream<bool>,
    pub emotes_enabled: bool,
}

impl Builder {
    pub fn new(
        injector: &injector::Injector,
        message_log: message_log::MessageLog,
        twitch: Twitch,
        settings: settings::Settings,
    ) -> Result<Self, Error> {
        let (cache_stream, cache) = injector.stream::<Cache>();

        let (enabled_stream, enabled) = settings.stream("enabled").or_default()?;

        let (emotes_enabled_stream, emotes_enabled) =
            settings.stream("emotes-enabled").or_default()?;

        message_log.enabled(enabled);

        Ok(Self {
            message_log,
            twitch,
            cache_stream,
            cache,
            enabled_stream,
            enabled,
            emotes_enabled_stream,
            emotes_enabled,
        })
    }

    /// Set if this is enabled or not.
    pub fn enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.message_log.enabled(enabled);
    }

    /// Construct a new chat log with the specified configuration.
    pub fn build(&self) -> Result<Option<ChatLog>, Error> {
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
    pub async fn observe(&self, tags: &irc::Tags, channel: &Channel, name: &str, message: &str) {
        let rendered = match self.emotes.as_ref() {
            Some(emotes) => match emotes.render(&tags, channel, name, message).await {
                Ok(rendered) => Some(rendered),
                Err(e) => {
                    log::warn!("failed to render emotes: {}", e);
                    None
                }
            },
            None => None,
        };

        self.message_log.push_back(&tags, &name, message, rendered);
    }
}
