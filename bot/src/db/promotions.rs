use crate::{db, template, utils};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use failure::{format_err, ResultExt as _};
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, err_derive::Error)]
pub enum BumpError {
    /// Trying to bump something which doesn't exist.
    #[error(display = "promotion missing")]
    Missing,
    /// Database error occurred.
    #[error(display = "database error: {}", _0)]
    Database(failure::Error),
}

/// The db of a words store.
trait Backend: Clone + Send + Sync {
    /// List all promos in db.
    fn list(&self) -> Result<Vec<db::models::Promotion>, failure::Error>;

    /// Insert or update an existing promotion.
    fn edit(&self, key: &Key, frequency: utils::Duration, text: &str)
        -> Result<(), failure::Error>;

    /// Delete the given promotion from the db.
    fn delete(&self, key: &Key) -> Result<bool, failure::Error>;

    /// Rename the given promotion.
    fn rename(&self, from: &Key, to: &Key) -> Result<bool, failure::Error>;

    /// Bump when the given promotion was last run.
    fn bump_promoted_at(&self, from: &Key, now: &DateTime<Utc>) -> Result<bool, failure::Error>;
}

impl Backend for db::Database {
    fn edit(
        &self,
        key: &Key,
        frequency: utils::Duration,
        text: &str,
    ) -> Result<(), failure::Error> {
        use db::schema::promotions::dsl;

        let c = self.pool.lock();
        let filter =
            dsl::promotions.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));
        let b = filter
            .clone()
            .first::<db::models::Promotion>(&*c)
            .optional()?;

        let frequency = frequency.num_seconds() as i32;

        match b {
            None => {
                let command = db::models::Promotion {
                    channel: key.channel.to_string(),
                    name: key.name.to_string(),
                    frequency,
                    promoted_at: None,
                    text: text.to_string(),
                };

                diesel::insert_into(dsl::promotions)
                    .values(&command)
                    .execute(&*c)?;
            }
            Some(_) => {
                diesel::update(filter)
                    .set((dsl::text.eq(text), dsl::frequency.eq(frequency)))
                    .execute(&*c)?;
            }
        }

        Ok(())
    }

    fn delete(&self, key: &Key) -> Result<bool, failure::Error> {
        use db::schema::promotions::dsl;

        let c = self.pool.lock();
        let count = diesel::delete(
            dsl::promotions.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
        )
        .execute(&*c)?;
        Ok(count == 1)
    }

    fn list(&self) -> Result<Vec<db::models::Promotion>, failure::Error> {
        use db::schema::promotions::dsl;
        let c = self.pool.lock();
        Ok(dsl::promotions.load::<db::models::Promotion>(&*c)?)
    }

    fn rename(&self, from: &Key, to: &Key) -> Result<bool, failure::Error> {
        use db::schema::promotions::dsl;

        let c = self.pool.lock();
        let count = diesel::update(
            dsl::promotions.filter(dsl::channel.eq(&from.channel).and(dsl::name.eq(&from.name))),
        )
        .set((dsl::channel.eq(&to.channel), dsl::name.eq(&to.name)))
        .execute(&*c)?;

        Ok(count == 1)
    }

    fn bump_promoted_at(&self, from: &Key, now: &DateTime<Utc>) -> Result<bool, failure::Error> {
        use db::schema::promotions::dsl;

        let c = self.pool.lock();
        let count = diesel::update(
            dsl::promotions.filter(dsl::channel.eq(&from.channel).and(dsl::name.eq(&from.name))),
        )
        .set(dsl::promoted_at.eq(now.naive_utc()))
        .execute(&*c)?;

        Ok(count == 1)
    }
}

#[derive(Clone)]
pub struct Promotions {
    inner: Arc<RwLock<HashMap<Key, Arc<Promotion>>>>,
    db: db::Database,
}

impl Promotions {
    /// Construct a new promos store with a db.
    pub fn load(db: db::Database) -> Result<Promotions, failure::Error> {
        let mut inner = HashMap::new();

        for promotion in db.list()? {
            let template = template::Template::compile(&promotion.text).with_context(|_| {
                format_err!("failed to compile promotion `{:?}` from db", promotion)
            })?;

            let key = Key::new(promotion.channel.as_str(), promotion.name.as_str());
            let frequency = utils::Duration::seconds(promotion.frequency as u64);
            let promoted_at = promotion
                .promoted_at
                .map(|d| DateTime::<Utc>::from_utc(d, Utc));

            inner.insert(
                key.clone(),
                Arc::new(Promotion {
                    key,
                    frequency,
                    promoted_at,
                    template,
                }),
            );
        }

        Ok(Promotions {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub fn edit(
        &self,
        channel: &str,
        name: &str,
        frequency: utils::Duration,
        text: &str,
    ) -> Result<(), failure::Error> {
        let key = Key::new(channel, name);

        let template = template::Template::compile(text)?;
        self.db.edit(&key, frequency.clone(), text)?;

        let mut inner = self.inner.write();

        inner.insert(
            key.clone(),
            Arc::new(Promotion {
                key,
                frequency,
                template,
                promoted_at: None,
            }),
        );

        Ok(())
    }

    /// Remove promotion.
    pub fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        let key = Key::new(channel, name);

        if !self.db.delete(&key)? {
            return Ok(false);
        }

        self.inner.write().remove(&key);
        Ok(true)
    }

    /// Test the given word.
    pub fn get<'a>(&'a self, channel: &str, name: &str) -> Option<Arc<Promotion>> {
        let key = Key::new(channel, name);

        let inner = self.inner.read();

        if let Some(promotion) = inner.get(&key) {
            return Some(Arc::clone(promotion));
        }

        None
    }

    /// Get a list of all promos.
    pub fn list(&self, channel: &str) -> Vec<Arc<Promotion>> {
        let inner = self.inner.read();

        let mut out = Vec::new();

        for c in inner.values() {
            if c.key.channel != channel {
                continue;
            }

            out.push(Arc::clone(c));
        }

        out
    }

    /// Try to rename the promotion.
    pub fn rename(&self, channel: &str, from: &str, to: &str) -> Result<(), db::RenameError> {
        let from = Key::new(channel, from);
        let to = Key::new(channel, to);

        let mut inner = self.inner.write();

        if inner.contains_key(&to) {
            return Err(db::RenameError::Conflict);
        }

        let promotion = match inner.remove(&from) {
            Some(promotion) => promotion,
            None => return Err(db::RenameError::Missing),
        };

        let promotion = Promotion {
            key: to.clone(),
            frequency: promotion.frequency.clone(),
            template: promotion.template.clone(),
            promoted_at: promotion.promoted_at,
        };

        match self.db.rename(&from, &to) {
            Err(e) => {
                log::error!(
                    "failed to rename promotion `{}` in database: {}",
                    from.name,
                    e
                );
            }
            Ok(false) => {
                log::warn!("promotion {} not renamed in database", from.name);
            }
            Ok(true) => (),
        }

        inner.insert(to, Arc::new(promotion));
        Ok(())
    }

    /// Bump that the given promotion was last promoted right now.
    pub fn bump_promoted_at(&self, promotion: &Promotion) -> Result<(), BumpError> {
        let mut inner = self.inner.write();

        let promotion = match inner.remove(&promotion.key) {
            Some(promotion) => promotion,
            None => return Err(BumpError::Missing),
        };

        let now = Utc::now();

        self.db
            .bump_promoted_at(&promotion.key, &now)
            .map_err(BumpError::Database)?;

        let promotion = Promotion {
            key: promotion.key.clone(),
            frequency: promotion.frequency.clone(),
            template: promotion.template.clone(),
            promoted_at: Some(now),
        };

        inner.insert(promotion.key.clone(), Arc::new(promotion));
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Key {
    pub channel: String,
    pub name: String,
}

impl Key {
    pub fn new(channel: &str, name: &str) -> Self {
        Self {
            channel: channel.to_string(),
            name: name.to_lowercase(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Promotion {
    pub key: Key,
    pub frequency: utils::Duration,
    pub promoted_at: Option<DateTime<Utc>>,
    pub template: template::Template,
}

impl Promotion {
    /// Render the given promotion.
    pub fn render<T>(&self, data: &T) -> Result<String, failure::Error>
    where
        T: serde::Serialize,
    {
        Ok(self.template.render_to_string(data)?)
    }
}
