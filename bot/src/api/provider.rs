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

use anyhow::Result;

use crate::api;
use crate::oauth2;
use crate::prelude::*;
use crate::tags;

/// The injected user information.
#[derive(Debug)]
pub(crate) struct User {
    /// Identifier of the user.
    pub(crate) id: String,
    /// The login of the user.
    pub(crate) login: String,
    /// The display name of the user.
    pub(crate) display_name: String,
}

impl User {
    fn from_api(api: api::twitch::new::User) -> Self {
        Self {
            id: api.id,
            login: api.login,
            display_name: api.display_name,
        }
    }
}

/// The injected structure for a connected twitch client.
#[derive(Clone)]
pub(crate) struct TwitchAndUser {
    /// The user the connection refers to.
    pub(crate) user: Arc<api::User>,
    /// The client connection.
    pub(crate) client: api::Twitch,
}

/// Set up the task to provide various twitch clients.
pub(crate) async fn twitch_clients_task(injector: Injector) -> Result<()> {
    let streamer = TwitchAndUserProvider::run(injector.clone(), tags::Twitch::Streamer);
    let bot = TwitchAndUserProvider::run(injector.clone(), tags::Twitch::Bot);
    tokio::try_join!(streamer, bot)?;
    Ok(())
}

struct TwitchAndUserProvider {
    key: Key<TwitchAndUser>,
    injector: Injector,
    /// Currently known user id.
    user_id: Option<Box<str>>,
}

impl TwitchAndUserProvider {
    pub(crate) async fn run(injector: Injector, id: tags::Twitch) -> Result<()> {
        let (mut token_stream, token) = injector
            .stream_key(&Key::<oauth2::SyncToken>::tagged(tags::Token::Twitch(id))?)
            .await;

        let mut this = TwitchAndUserProvider {
            key: Key::<TwitchAndUser>::tagged(id)?,
            injector,
            user_id: None,
        };

        this.update(token).await?;

        // loop to setup all necessary twitch authentication.
        loop {
            let token = token_stream.recv().await;
            this.update(token).await?;
        }
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

        let user = match client.user().await {
            Ok(user) => user,
            Err(e) => {
                client.token.force_refresh().await?;
                log_warn!(e, "failed to get twitch user information");
                return Ok(());
            }
        };

        let user = User::from_api(user);

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
                    client,
                },
            )
            .await;

        Ok(())
    }
}
