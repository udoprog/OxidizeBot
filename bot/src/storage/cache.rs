use crate::{prelude::*, timer::Interval, utils};
use ccl::dhashmap::DHashMap;
use chrono::{DateTime, Duration, Utc};
use crossbeam::queue::SegQueue;
use failure::Error;
use hashbrown::HashSet;
use hex::ToHex as _;
use serde_cbor as cbor;
use serde_json as json;
use std::{fmt, sync::Arc, time};
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
    expired: SegQueue<Vec<u8>>,
    /// Entries inserted.
    inserted: SegQueue<Vec<u8>>,
    /// Underlying storage.
    map: DHashMap<Vec<u8>, Entry>,
}

#[derive(Clone)]
pub struct Cache {
    ns: Option<Arc<String>>,
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
                let entry: Entry = match cbor::from_slice(&*value) {
                    Ok(entry) => entry,
                    Err(e) => {
                        if log::log_enabled!(log::Level::Trace) {
                            log::warn!(
                                "{}: failed to load: {}: {}",
                                KeyFormat(&*key),
                                e,
                                KeyFormat(&*value)
                            );
                        } else {
                            log::warn!("{}: failed to load: {}", KeyFormat(&*key), e);
                        }

                        expired.push(key.to_vec());
                        continue;
                    }
                };

                if entry.is_expired(now) {
                    expired.push(key.to_vec());
                    continue;
                }

                map.insert(key.to_vec(), entry);
            }

            Inner {
                db,
                expired,
                inserted: Default::default(),
                map,
            }
        };

        let cache = Cache {
            ns: None,
            inner: Arc::new(inner),
        };

        cache.cleanup()?;
        Ok(cache)
    }

    /// Create a namespaced cache.
    ///
    /// The namespace must be unique to avoid conflicts.
    pub fn namespaced(&self, ns: impl AsRef<str>) -> Self {
        Self {
            ns: Some(Arc::new(ns.as_ref().to_string())),
            inner: self.inner.clone(),
        }
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
                    .map(|k| KeyFormat(k).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if log::log_enabled!(log::Level::Trace) && !to_insert.is_empty() {
            log::trace!(
                "Inserting: {}",
                to_insert
                    .iter()
                    .map(|k| KeyFormat(k).to_string())
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

    /// Helper to serialize the key with a namespace.
    fn key<T>(&self, key: T) -> Result<Vec<u8>, Error>
    where
        T: serde::Serialize,
    {
        let key = match self.ns.as_ref() {
            Some(ns) => Key(Some(&*ns), key),
            None => Key(None, key),
        };

        return cbor::to_vec(&key).map_err(Into::into);

        #[derive(serde::Serialize)]
        struct Key<'a, T>(Option<&'a str>, T);
    }

    /// Insert a value into the cache.
    pub fn insert<K, T>(&self, key: K, age: Duration, value: T) -> Result<(), Error>
    where
        K: serde::Serialize,
        T: serde::Serialize,
    {
        let key = self.key(&key)?;
        self.inner_insert(&key, age, value)
    }

    /// Insert a value into the cache.
    fn inner_insert<T>(&self, key: &Vec<u8>, age: Duration, value: T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let value = match cbor::value::to_value(value) {
            Ok(value) => value,
            Err(e) => {
                log::trace!("store:{} *errored*", KeyFormat(key));
                return Err(e.into());
            }
        };

        log::trace!("store:{}", KeyFormat(key));

        let expires_at = Utc::now() + age;
        self.inner
            .map
            .insert(key.to_vec(), Entry { expires_at, value });
        self.inner.inserted.push(key.to_vec());
        Ok(())
    }

    /// Load an entry from the cache.
    pub fn get<K, T>(&self, key: K) -> Result<Option<T>, Error>
    where
        K: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let key = self.key(&key)?;
        self.inner_get(&key)
    }

    /// Load an entry from the cache.
    fn inner_get<T>(&self, key: &Vec<u8>) -> Result<Option<T>, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let value = {
            let entry = match self.inner.map.get(key) {
                Some(entry) => entry,
                None => {
                    log::trace!("load:{} -> null (missing)", KeyFormat(key));
                    return Ok(None);
                }
            };

            if entry.is_expired(Utc::now()) {
                log::trace!("load:{} -> null (expired)", KeyFormat(key));
                self.inner.expired.push(key.to_vec());
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
                            log::warn!("{}: failed to load: {}: {}", KeyFormat(key), e, value);
                        }
                        Err(string_e) => {
                            log::warn!(
                                "{}: failed to load: {}: *failed to serialize*: {}",
                                KeyFormat(key),
                                e,
                                string_e
                            );
                        }
                    }
                } else {
                    log::warn!("{}: failed to load: {}", KeyFormat(key), e);
                }

                log::trace!("load:{} -> null (error)", KeyFormat(key));
                return Ok(None);
            }
        };

        log::trace!("load:{} -> *value*", KeyFormat(key));
        Ok(Some(value))
    }

    /// Wrap the result of the given future to load and store from cache.
    pub async fn wrap<K, T>(
        &self,
        key: K,
        age: utils::Duration,
        future: impl Future<Output = Result<T, Error>>,
    ) -> Result<T, Error>
    where
        K: serde::Serialize,
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        let key = self.key(&key)?;

        if let Some(output) = self.inner_get(&key)? {
            return Ok(output);
        }

        let output = future.await?;
        self.inner_insert(&key, age.as_chrono(), output.clone())?;
        Ok(output)
    }
}

/// Helper formatter to convert cbor bytes to JSON or hex.
struct KeyFormat<'a>(&'a [u8]);

impl fmt::Display for KeyFormat<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match cbor::from_slice::<cbor::Value>(self.0) {
            Ok(value) => value,
            Err(_) => return self.0.write_hex(fmt),
        };

        let value = match json::to_string(&value) {
            Ok(value) => value,
            Err(_) => return self.0.write_hex(fmt),
        };

        value.fmt(fmt)
    }
}
