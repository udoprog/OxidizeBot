//! Stream currency configuration.
use crate::{db, twitch};
use futures::Future;
use hashbrown::HashSet;
use std::sync::Arc;

/// Configuration for a currency.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub name: String,
}

impl Config {
    pub fn into_currency(self, db: db::Database, twitch: twitch::Twitch) -> Currency {
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
    twitch: twitch::Twitch,
}

impl Currency {
    /// Reward all users.
    pub fn add_channel_all(
        &self,
        channel: &str,
        reward: i64,
    ) -> impl Future<Item = usize, Error = failure::Error> {
        self.twitch
            .chatters(channel)
            .and_then(|chatters| {
                let mut u = HashSet::new();
                u.extend(chatters.viewers);
                u.extend(chatters.moderators);
                u.extend(chatters.broadcaster);
                Ok(u)
            })
            // update database.
            .and_then({
                let channel = channel.to_string();
                let db = self.db.clone();

                move |users| {
                    let len = users.len();
                    db.balances_increment(channel.as_str(), users, reward)
                        .map(move |_| len)
                }
            })
    }
}
