use crate::{
    db::{self, models},
    prelude::*,
    timer::Interval,
    utils,
};
use chrono::{DateTime, Duration, Utc};
use failure::Error;
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::{collections::VecDeque, sync::Arc, time};
use tokio_threadpool::ThreadPool;

use diesel::prelude::*;

struct Entry {
    expires_at: DateTime<Utc>,
    value: serde_json::Value,
}

struct Entries {
    inserted: VecDeque<String>,
    map: HashMap<String, Entry>,
}

struct Inner {
    db: db::Database,
    /// Current entries.
    entries: RwLock<Entries>,
}

#[derive(Clone)]
pub struct Cache(Arc<Inner>);

impl Cache {
    /// Load the cache from the database.
    pub fn load(db: db::Database) -> Result<Cache, Error> {
        use db::schema::cache::dsl;

        let entries = {
            let c = db.pool.lock();
            dsl::cache.load::<models::CacheEntry>(&*c)?
        };

        let entries = {
            let mut map = HashMap::new();

            for entry in entries {
                let expires_at = DateTime::from_utc(entry.expires_at, Utc);

                let value = match serde_json::from_str(&entry.value) {
                    Ok(value) => value,
                    Err(e) => {
                        log::warn!("{}: failed to deserialize: {}", entry.key, e);
                        continue;
                    }
                };

                map.insert(entry.key, Entry { expires_at, value });
            }

            Entries {
                inserted: Default::default(),
                map,
            }
        };

        let inner = Inner {
            db,
            entries: RwLock::new(entries),
        };

        Ok(Cache(Arc::new(inner)))
    }

    /// Store and clean cache.
    fn store_and_load(&self) -> Result<(), Error> {
        use db::schema::cache::dsl;

        let now = Utc::now().naive_utc();

        let c = self.0.db.pool.lock();
        let mut entries = self.0.entries.write();

        while let Some(insert) = entries.inserted.pop_front() {
            if let Some(entry) = entries.map.get(&insert) {
                let filter = dsl::cache.filter(dsl::key.eq(&insert));

                let e = filter.clone().first::<models::CacheEntry>(&*c).optional()?;

                match e {
                    None => {
                        let entry = models::CacheEntry {
                            key: insert,
                            expires_at: entry.expires_at.naive_utc(),
                            value: serde_json::to_string(&entry.value)?,
                        };

                        diesel::insert_into(dsl::cache).values(entry).execute(&*c)?;
                    }
                    Some(_) => {
                        let set = models::UpdateCacheEntry {
                            value: serde_json::to_string(&entry.value)?,
                        };

                        diesel::update(filter).set(&set).execute(&*c)?;
                    }
                }
            }
        }

        let mut to_delete;

        // delete from database.
        {
            to_delete = dsl::cache
                .select(dsl::key)
                .filter(dsl::expires_at.lt(&now))
                .load::<String>(&*c)?;

            if log::log_enabled!(log::Level::Debug) && to_delete.len() > 0 {
                log::trace!("Deleting expired: {}", to_delete.join(", "));
            }

            diesel::delete(dsl::cache.filter(dsl::key.eq_any(&to_delete))).execute(&*c)?;
        }

        // delete expired local entries.
        for key in to_delete {
            entries.map.remove(&key);
        }

        Ok(())
    }

    /// Run the cache loop.
    pub async fn run(self) -> Result<(), Error> {
        let thread_pool = ThreadPool::new();
        let mut interval = Interval::new_interval(time::Duration::from_secs(60 * 5));

        loop {
            futures::select! {
                _ = interval.select_next_some() => {
                    let cache = self.clone();

                    let future = thread_pool.spawn_handle(future01::lazy(move || {
                        cache.store_and_load()
                    }));

                    future.compat().await?;
                }
            }
        }
    }

    /// Insert a value into the cache.
    pub fn insert<T>(&self, key: String, age: Duration, value: T) -> Result<(), Error>
    where
        T: serde::ser::Serialize,
    {
        let value = match serde_json::to_value(value) {
            Ok(value) => value,
            Err(e) => {
                log::trace!("store:{} *errored*", key);
                return Err(e.into());
            }
        };

        log::trace!("store:{}", key);

        let expires_at = Utc::now() + age;
        let mut entries = self.0.entries.write();
        entries.inserted.push_back(key.clone());
        entries.map.insert(key, Entry { expires_at, value });
        Ok(())
    }

    /// Load an entry from the cache.
    pub fn get<T>(&self, key: String) -> Result<Option<T>, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let value;

        {
            let entries = self.0.entries.read();

            let entry = match entries.map.get(&key) {
                Some(entry) => entry,
                None => {
                    log::trace!("load:{} -> null (missing)", key);
                    return Ok(None);
                }
            };

            if entry.expires_at < Utc::now() {
                log::trace!("load:{} -> null (expired)", key);
                return Ok(None);
            }

            value = entry.value.clone();
        }

        let value = match serde_json::from_value(value.clone()) {
            Ok(value) => value,
            Err(e) => {
                log::warn!("{}: failed to deserialize: {}", key, e);
                log::trace!("load:{} -> null (error)", key);
                return Ok(None);
            }
        };

        log::trace!("load:{} -> *value*", key);
        Ok(Some(value))
    }

    /// Wrap the result of the given future to load and store from cache.
    pub async fn wrap<T>(
        &self,
        key: String,
        age: utils::Duration,
        future: impl Future<Output = Result<T, Error>>,
    ) -> Result<T, Error>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        if let Some(output) = self.get(key.clone())? {
            return Ok(output);
        }

        let output = future.await?;
        self.insert(key, age.as_chrono(), output.clone())?;
        Ok(output)
    }
}
