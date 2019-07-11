use crate::{prelude::*, timer::Interval, utils};
use ccl::dhashmap::DHashMap;
use chrono::{DateTime, Duration, Utc};
use crossbeam::queue::SegQueue;
use failure::Error;
use hashbrown::HashSet;
use serde_cbor as cbor;
use serde_json as json;
use std::{sync::Arc, time};
use tokio_threadpool::ThreadPool;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Entry {
    expires_at: DateTime<Utc>,
    value: cbor::Value,
}

impl Entry {
    /// Test if entry is expired.
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at < now
    }
}

struct Inner {
    db: Arc<rocksdb::DB>,
    /// Entries marked as expired after trying to access them.
    expired: SegQueue<String>,
    /// Entries inserted.
    inserted: SegQueue<String>,
    /// Underlying storage.
    map: DHashMap<String, Entry>,
}

#[derive(Clone)]
pub struct Cache {
    inner: Arc<Inner>,
}

impl Cache {
    /// Load the cache from the database.
    pub fn load(db: Arc<rocksdb::DB>) -> Result<Cache, Error> {
        let now = Utc::now();

        let inner = {
            let map = DHashMap::default();
            let expired = SegQueue::default();

            for (key, value) in db.iterator(rocksdb::IteratorMode::Start) {
                let key = std::str::from_utf8(&*key)?;

                let entry: Entry = match cbor::from_slice(&*value) {
                    Ok(entry) => entry,
                    Err(e) => {
                        if log::log_enabled!(log::Level::Trace) {
                            log::warn!(
                                "{}: failed to load: {}: {}",
                                key,
                                e,
                                String::from_utf8_lossy(&*value)
                            );
                        } else {
                            log::warn!("{}: failed to load: {}", key, e);
                        }

                        expired.push(key.to_string());
                        continue;
                    }
                };

                if entry.is_expired(now) {
                    expired.push(key.to_string());
                    continue;
                }

                map.insert(key.to_string(), entry);
            }

            Inner {
                db,
                expired,
                inserted: Default::default(),
                map,
            }
        };

        let cache = Cache {
            inner: Arc::new(inner),
        };

        cache.cleanup()?;
        Ok(cache)
    }

    /// Store and clean cache.
    fn cleanup(&self) -> Result<(), Error> {
        let mut to_delete = HashSet::new();
        let mut to_insert = HashSet::new();

        while let Ok(delete) = self.inner.expired.pop() {
            to_delete.insert(delete);
        }

        while let Ok(insert) = self.inner.inserted.pop() {
            to_delete.remove(&insert);
            to_insert.insert(insert);
        }

        if to_delete.is_empty() && to_insert.is_empty() {
            return Ok(());
        }

        if log::log_enabled!(log::Level::Trace) && !to_delete.is_empty() {
            log::trace!(
                "Deleting: {}",
                to_delete
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if log::log_enabled!(log::Level::Trace) && !to_insert.is_empty() {
            log::trace!(
                "Inserting: {}",
                to_insert
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        for key in to_delete {
            self.inner.db.delete(&key)?;
            self.inner.map.remove(&key);
        }

        for insert in to_insert {
            if let Some(entry) = self.inner.map.get(&insert) {
                let value = cbor::to_vec(&*entry)?;
                self.inner.db.put(&insert, &value)?;
            }
        }

        log::trace!("compacting database");
        self.inner.db.compact_range::<[u8; 0], [u8; 0]>(None, None);
        Ok(())
    }

    /// Run the cache loop.
    pub async fn run(self) -> Result<(), Error> {
        let thread_pool = ThreadPool::new();
        let mut interval = Interval::new_interval(time::Duration::from_secs(10));

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
        let value = match cbor::value::to_value(value) {
            Ok(value) => value,
            Err(e) => {
                log::trace!("store:{} *errored*", key);
                return Err(e.into());
            }
        };

        log::trace!("store:{}", key);

        let expires_at = Utc::now() + age;
        self.inner
            .map
            .insert(key.clone(), Entry { expires_at, value });
        self.inner.inserted.push(key);
        Ok(())
    }

    /// Load an entry from the cache.
    pub fn get<T>(&self, key: String) -> Result<Option<T>, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let value = {
            let entry = match self.inner.map.get(&key) {
                Some(entry) => entry,
                None => {
                    log::trace!("load:{} -> null (missing)", key);
                    return Ok(None);
                }
            };

            if entry.is_expired(Utc::now()) {
                log::trace!("load:{} -> null (expired)", key);
                self.inner.expired.push(key);
                return Ok(None);
            }

            entry.value.clone()
        };

        let value = match cbor::value::from_value(value.clone()) {
            Ok(value) => value,
            Err(e) => {
                if log::log_enabled!(log::Level::Trace) {
                    match json::to_string(&value) {
                        Ok(value) => {
                            log::warn!("{}: failed to load: {}: {}", key, e, value);
                        }
                        Err(string_e) => {
                            log::warn!(
                                "{}: failed to load: {}: *failed to serialize*: {}",
                                key,
                                e,
                                string_e
                            );
                        }
                    }
                } else {
                    log::warn!("{}: failed to load: {}", key, e);
                }

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
