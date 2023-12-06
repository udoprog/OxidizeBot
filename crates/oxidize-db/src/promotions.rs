use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use common::{Channel, Duration, OwnedChannel};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum BumpError {
    /// Trying to bump something which doesn't exist.
    #[error("promotion missing")]
    Missing,
    /// Database error occurred.
    #[error("database error: {}", _0)]
    Database(#[source] anyhow::Error),
}

/// Local database wrapper.
#[derive(Clone)]
struct Database(crate::Database);

impl Database {
    private_database_group_fns!(promotions, Promotion, Key);

    async fn edit(
        &self,
        key: &Key,
        frequency: Duration,
        text: &str,
    ) -> Result<Option<crate::models::Promotion>> {
        use crate::schema::promotions::dsl;

        let key = key.clone();
        let text = text.to_string();

        self.0
            .asyncify(move |c| {
                let filter = dsl::promotions
                    .filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));
                let b = filter.first::<crate::models::Promotion>(c).optional()?;

                let frequency = frequency.num_seconds() as i32;

                match b {
                    None => {
                        let command = crate::models::Promotion {
                            channel: key.channel.to_owned(),
                            name: key.name.to_string(),
                            frequency,
                            promoted_at: None,
                            text: text.to_string(),
                            group: None,
                            disabled: false,
                        };

                        diesel::insert_into(dsl::promotions)
                            .values(&command)
                            .execute(c)?;

                        Ok(None)
                    }
                    Some(promotion) => {
                        let mut set = crate::models::UpdatePromotion::default();
                        set.text = Some(&text);
                        set.frequency = Some(frequency);

                        diesel::update(filter).set(&set).execute(c)?;

                        if promotion.disabled {
                            return Ok(None);
                        }

                        Ok(Some(promotion))
                    }
                }
            })
            .await
    }

    async fn bump_promoted_at(&self, from: &Key, now: &DateTime<Utc>) -> Result<bool> {
        use crate::schema::promotions::dsl;

        let from = from.clone();
        let now = *now;

        self.0
            .asyncify(move |c| {
                let count = diesel::update(
                    dsl::promotions
                        .filter(dsl::channel.eq(&from.channel).and(dsl::name.eq(&from.name))),
                )
                .set(dsl::promoted_at.eq(now.naive_utc()))
                .execute(c)?;

                Ok(count == 1)
            })
            .await
    }
}

#[derive(Clone)]
pub struct Promotions {
    inner: Arc<RwLock<HashMap<Key, Arc<Promotion>>>>,
    db: Database,
}

impl Promotions {
    database_group_fns!(Promotion, Key);

    /// Construct a new promos store with a db.
    pub async fn load(db: crate::Database) -> Result<Promotions> {
        let db = Database(db);

        let mut inner = HashMap::new();

        for promotion in db.list().await? {
            let promotion = Promotion::from_db(&promotion)?;
            inner.insert(promotion.key.clone(), Arc::new(promotion));
        }

        Ok(Promotions {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub async fn edit(
        &self,
        channel: &Channel,
        name: &str,
        frequency: Duration,
        template: template::Template,
    ) -> Result<()> {
        let key = Key::new(channel, name);

        let mut inner = self.inner.write().await;

        if let Some(promotion) = self.db.edit(&key, frequency, template.source()).await? {
            let promoted_at = promotion
                .promoted_at
                .map(|d| DateTime::from_naive_utc_and_offset(d, Utc));

            inner.insert(
                key.clone(),
                Arc::new(Promotion {
                    key,
                    frequency,
                    promoted_at,
                    template,
                    group: promotion.group,
                    disabled: promotion.disabled,
                }),
            );
        } else {
            inner.remove(&key);
        }

        Ok(())
    }

    /// Bump that the given promotion was last promoted right now.
    #[tracing::instrument(skip_all)]
    pub async fn bump_promoted_at(&self, promotion: &Promotion) -> Result<(), BumpError> {
        let mut inner = self.inner.write().await;

        let promotion = match inner.remove(&promotion.key) {
            Some(promotion) => promotion,
            None => return Err(BumpError::Missing),
        };

        let now = Utc::now();

        self.db
            .bump_promoted_at(&promotion.key, &now)
            .await
            .map_err(BumpError::Database)?;

        let mut promotion = (*promotion).clone();
        promotion.promoted_at = Some(now);

        inner.insert(promotion.key.clone(), Arc::new(promotion));
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key {
    pub channel: OwnedChannel,
    pub name: String,
}

impl Key {
    pub(crate) fn new(channel: &Channel, name: &str) -> Self {
        Self {
            channel: channel.to_owned(),
            name: name.to_lowercase(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Promotion {
    pub key: Key,
    pub frequency: Duration,
    pub promoted_at: Option<DateTime<Utc>>,
    pub template: template::Template,
    pub group: Option<String>,
    pub disabled: bool,
}

impl Promotion {
    pub(crate) const NAME: &'static str = "promotion";

    pub(crate) fn from_db(promotion: &crate::models::Promotion) -> Result<Promotion> {
        let template = template::Template::compile(&promotion.text)
            .with_context(|| anyhow!("failed to compile promotion `{:?}` from db", promotion))?;

        let key = Key::new(&promotion.channel, &promotion.name);
        let frequency = Duration::seconds(promotion.frequency as u64);
        let promoted_at = promotion
            .promoted_at
            .map(|d| DateTime::<Utc>::from_naive_utc_and_offset(d, Utc));

        Ok(Promotion {
            key,
            frequency,
            promoted_at,
            template,
            group: promotion.group.clone(),
            disabled: promotion.disabled,
        })
    }

    /// Render the given promotion.
    pub fn render<T>(&self, data: &T) -> Result<String>
    where
        T: Serialize,
    {
        self.template.render_to_string(data)
    }
}

impl fmt::Display for Promotion {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "frequency = {frequency}, template = \"{template}\", group = {group}, disabled = {disabled}",
            frequency = self.frequency,
            template = self.template,
            group = self.group.as_deref().unwrap_or("*none*"),
            disabled = self.disabled,
        )
    }
}
