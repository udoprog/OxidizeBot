//! Module to provide instances of `TwitchAndUser`, which are twitch clients
//! where we've successfully been able to call the user endpoint.
//!
//! This is provided anytime a token is made available which has a different
//! user id than the previous one, to force any downstream listeners to
//! reconfigure themselves if the user some component has been configured for
//! changes.
//!
//! Note: moving it out of the `irc` module now means that this can be used by
//! other bot components - not just chat modules.

use crate::api;
use crate::oauth2;
use crate::prelude::*;
use crate::tags;
use anyhow::Result;

/// The injected structure for a connected twitch client.
#[derive(Clone)]
pub struct TwitchAndUser {
    /// The user the connection refers to.
    pub user: Arc<api::twitch::v5::User>,
    /// Channel associated with the api client.
    pub channel: Option<Arc<api::twitch::v5::Channel>>,
    /// The client connection.
    pub client: api::Twitch,
}

/// Set up the task to provide various twitch clients.
pub async fn twitch_clients_task(injector: injector::Injector) -> Result<()> {
    let streamer = TwitchAndUserProvider::run(injector.clone(), tags::Twitch::Streamer, true);
    let bot = TwitchAndUserProvider::run(injector.clone(), tags::Twitch::Bot, false);
    tokio::try_join!(streamer, bot)?;
    Ok(())
}

struct TwitchAndUserProvider {
    key: injector::Key<TwitchAndUser>,
    injector: injector::Injector,
    /// Currently known user id.
    user_id: Option<Box<str>>,
    /// If we want to include channel information.
    channel: bool,
}

impl TwitchAndUserProvider {
    pub async fn run(injector: injector::Injector, id: tags::Twitch, channel: bool) -> Result<()> {
        let (mut token_stream, token) = injector
            .stream_key(&injector::Key::<oauth2::SyncToken>::tagged(
                tags::Token::Twitch(id),
            )?)
            .await;

        let mut this = TwitchAndUserProvider {
            key: injector::Key::<TwitchAndUser>::tagged(id)?,
            injector,
            user_id: None,
            channel,
        };

        this.update(token).await?;

        // loop to setup all necessary twitch authentication.
        if let Some(token) = token_stream.next().await {
            this.update(token).await?;
        }

        Ok(())
    }

    /// Inner update helper function.
    async fn update(&mut self, token: Option<oauth2::SyncToken>) -> Result<()> {
        let token = match token {
            Some(token) => token,
            None => {
                let _ = self.injector.clear_key(&self.key).await;
                return Ok(());
            }
        };

        // Construct a client wrapping the new token and fetch user. Compare the
        // user with the one that is locally known and update injections
        // accordingly.
        let client = api::Twitch::new(token)?;

        let (user, channel) = if self.channel {
            match tokio::try_join!(client.v5_user(), client.v5_channel()) {
                Ok((user, channel)) => (user, Some(channel)),
                Err(e) => {
                    client.token.force_refresh().await?;
                    log_warn!(e, "failed to get twitch user information");
                    return Ok(());
                }
            }
        } else {
            match client.v5_user().await {
                Ok(ok) => (ok, None),
                Err(e) => {
                    client.token.force_refresh().await?;
                    log_warn!(e, "failed to get twitch user information");
                    return Ok(());
                }
            }
        };

        if Some(user.id.as_str()) == self.user_id.as_deref() {
            // Client w/ same user id. Do not update.
            return Ok(());
        }

        self.user_id = Some(user.id.clone().into());

        let _ = self
            .injector
            .update_key(
                &self.key,
                TwitchAndUser {
                    user: Arc::new(user),
                    channel: channel.map(Arc::new),
                    client,
                },
            )
            .await;

        Ok(())
    }
}
