//! Stream currency configuration.
use crate::{api, db};
use hashbrown::HashSet;
use std::sync::Arc;

/// Configuration for a currency.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub name: String,
}

impl Config {
    pub fn into_currency(self, db: db::Database, twitch: api::Twitch) -> Currency {
        Currency {
            name: Arc::new(self.name),
            db,
            twitch,
        }
    }
}

/// The currency being used.
#[derive(Clone)]
pub struct Currency {
    pub name: Arc<String>,
    db: db::Database,
    twitch: api::Twitch,
}

impl Currency {
    /// Reward all users.
    pub async fn add_channel_all(
        &self,
        channel: String,
        reward: i64,
    ) -> Result<usize, failure::Error> {
        let chatters = self.twitch.chatters(channel.clone()).await?;

        let mut users = HashSet::new();
        users.extend(chatters.viewers);
        users.extend(chatters.moderators);
        users.extend(chatters.broadcaster);

        let len = users.len();
        self.db.balances_increment(channel, users, reward).await?;
        Ok(len)
    }
}
