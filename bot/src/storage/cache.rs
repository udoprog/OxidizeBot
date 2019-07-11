use crate::{prelude::*, timer::Interval, utils};
use chrono::{DateTime, Duration, Utc};
use failure::Error;
use hashbrown::HashMap;
use parking_lot::RwLock;
use serde_json as json;
use std::{collections::VecDeque, sync::Arc, time};
use tokio_threadpool::ThreadPool;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Entry {
    expires_at: DateTime<Utc>,
    value: json::Value,
}

struct Entries {
    inserted: VecDeque<String>,
    map: HashMap<String, Entry>,
}

struct Inner {
    db: Arc<rocksdb::DB>,
    /// Current entries.
    entries: RwLock<Entries>,
}

#[derive(Clone)]
pub struct Cache {
    inner: Arc<Inner>,
}

impl Cache {
    /// Load the cache from the database.
    pub fn load(db: Arc<rocksdb::DB>) -> Result<Cache, Error> {
        let entries = {
            let mut map = HashMap::new();

            for (key, value) in db.iterator(rocksdb::IteratorMode::Start) {
                let key = std::str::from_utf8(&*key)?;
                let entry: Entry = json::from_slice(&*value)?;
                map.insert(key.to_string(), entry);
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

        Ok(Cache {
            inner: Arc::new(inner),
        })
    }

    /// Store and clean cache.
    fn cleanup(&self) -> Result<(), Error> {
        let now = Utc::now();

        let mut entries = self.inner.entries.write();

        while let Some(insert) = entries.inserted.pop_front() {
            if let Some(entry) = entries.map.get(&insert) {
                let value = json::to_vec(&entry)?;
                self.inner.db.put(&insert, &value)?;
            }
        }

        let to_delete: Vec<String> = entries
            .map
            .iter()
            .filter(|(_, e)| e.expires_at < now)
            .map(|(k, _)| k.to_string())
            .collect();

        if log::log_enabled!(log::Level::Debug) && to_delete.len() > 0 {
            log::trace!("Deleting expired: {}", to_delete.join(", "));
        }

        // delete expired local entries.
        for key in to_delete {
            self.inner.db.delete(&key)?;
            entries.map.remove(&key);
        }

        log::trace!("compacting database");
        self.inner.db.compact_range::<[u8; 0], [u8; 0]>(None, None);
        Ok(())
    }

    /// Run the cache loop.
    pub async fn run(self) -> Result<(), Error> {
        let thread_pool = ThreadPool::new();
        let mut interval = Interval::new_interval(time::Duration::from_secs(2 * 60));

        loop {
            futures::select! {
                _ = interval.select_next_some() => {
                    let cache = self.clone();

                    let future = thread_pool.spawn_handle(future01::lazy(move || {
                        cache.cleanup()
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
        let value = match json::to_value(value) {
            Ok(value) => value,
            Err(e) => {
                log::trace!("store:{} *errored*", key);
                return Err(e.into());
            }
        };

        log::trace!("store:{}", key);

        let expires_at = Utc::now() + age;
        let mut entries = self.inner.entries.write();
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
            let entries = self.inner.entries.read();

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

        let value = match json::from_value(value.clone()) {
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
