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

use std::sync::Arc;

use anyhow::Result;
use async_injector::{Injector, Key};
use common::tags;

use crate::token::Token;

/// The injected user information.
#[derive(Debug)]
pub struct User {
    /// Identifier of the user.
    pub id: String,
    /// The login of the user.
    pub login: String,
    /// The display name of the user.
    pub display_name: String,
}

impl User {
    fn from_api(api: crate::twitch::model::User) -> Self {
        Self {
            id: api.id,
            login: api.login,
            display_name: api.display_name,
        }
    }
}

/// The injected structure for a connected twitch client.
#[derive(Clone)]
pub struct TwitchAndUser {
    /// The user the connection refers to.
    pub user: Arc<crate::User>,
    /// The client connection.
    pub client: crate::Twitch,
}

/// Set up the task to provide various twitch clients.
#[tracing::instrument(skip(injector))]
pub async fn twitch_and_user(
    user_agent: &'static str,
    id: &'static str,
    tag: tags::Twitch,
    injector: Injector,
) -> Result<()> {
    let (mut token_stream, token) = injector
        .stream_key(&Key::<Token>::tagged(tags::Token::Twitch(tag))?)
        .await;

    let mut this = TwitchAndUserProvider {
        user_agent,
        key: Key::<TwitchAndUser>::tagged(tag)?,
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

struct TwitchAndUserProvider {
    user_agent: &'static str,
    key: Key<TwitchAndUser>,
    injector: Injector,
    /// Currently known user id.
    user_id: Option<Box<str>>,
}

impl TwitchAndUserProvider {
    /// Inner update helper function.
    async fn update(&mut self, token: Option<Token>) -> Result<()> {
        tracing::trace!("Updating with token");

        let token = match token {
            Some(token) if token.is_ready() => token,
            _ => {
                let _ = self.injector.clear_key(&self.key).await;
                return Ok(());
            }
        };

        // Construct a client wrapping the new token and fetch user. Compare the
        // user with the one that is locally known and update injections
        // accordingly.
        let client = crate::Twitch::new(self.user_agent, token)?;

        let user = match client.user().await {
            Ok(user) => user,
            Err(e) => {
                client.token().force_refresh();
                common::log_warn!(e, "failed to get twitch user information");
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
