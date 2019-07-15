use crate::{prelude::*, utils};
use chrono::{DateTime, Duration, Utc};
use failure::Error;
use hex::ToHex as _;
use serde_cbor as cbor;
use serde_json as json;
use std::{fmt, sync::Arc};

pub enum State<T> {
    /// Entry is fresh and can be used.
    Fresh(StoredEntry<T>),
    /// Entry exists, but is expired.
    /// Cache is referenced so that the value can be removed if needed.
    Expired(StoredEntry<T>),
    /// No entry.
    Missing,
}

/// Entry with a reference to the underlying cache.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonEntry {
    pub key: serde_json::Value,
    #[serde(flatten)]
    pub stored: StoredEntry<serde_json::Value>,
}

/// Entry with a reference to the underlying cache.
pub struct EntryRef<'a, T> {
    cache: &'a Cache,
    pub key: Vec<u8>,
    pub state: State<T>,
}

impl<'a, T> EntryRef<'a, T> {
    /// Create a fresh entry.
    pub fn fresh(cache: &'a Cache, key: Vec<u8>, stored: StoredEntry<T>) -> Self {
        EntryRef {
            cache,
            key,
            state: State::Fresh(stored),
        }
    }

    /// Create an expired entry.
    pub fn expired(cache: &'a Cache, key: Vec<u8>, stored: StoredEntry<T>) -> Self {
        EntryRef {
            cache,
            key,
            state: State::Expired(stored),
        }
    }

    /// Create a missing entry.
    pub fn missing(cache: &'a Cache, key: Vec<u8>) -> Self {
        EntryRef {
            cache,
            key,
            state: State::Missing,
        }
    }

    /// Get as an option, regardless if it's expired or not.
    pub fn get(self) -> Option<T> {
        match self.state {
            State::Fresh(e) | State::Expired(e) => Some(e.value),
            State::Missing => None,
        }
    }

    /// Get the value, but delete if it is expired.
    pub fn delete_if_expired(self) -> Result<Option<T>, Error> {
        match self.state {
            State::Fresh(e) => return Ok(Some(e.value)),
            State::Expired(..) => self.cache.db.delete(&self.key)?,
            State::Missing => (),
        }

        Ok(None)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StoredEntry<T> {
    expires_at: DateTime<Utc>,
    value: T,
}

impl<T> StoredEntry<T> {
    /// Test if entry is expired.
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at < now
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PartialStoredEntry {
    expires_at: DateTime<Utc>,
}

impl PartialStoredEntry {
    /// Test if entry is expired.
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at < now
    }

    /// Convert into a stored entry.
    fn into_stored_entry(self) -> StoredEntry<()> {
        StoredEntry {
            expires_at: self.expires_at,
            value: (),
        }
    }
}

#[derive(Clone)]
pub struct Cache {
    ns: Option<Arc<String>>,
    /// Underlying storage.
    db: Arc<rocksdb::DB>,
}

impl Cache {
    /// Load the cache from the database.
    pub fn load(db: Arc<rocksdb::DB>) -> Result<Cache, Error> {
        let cache = Cache { ns: None, db };
        cache.cleanup()?;
        Ok(cache)
    }

    /// Delete the given key from the specified namespace.
    pub fn delete_with_ns<K>(&self, ns: Option<&str>, key: K) -> Result<(), Error>
    where
        K: serde::Serialize,
    {
        let key = self.key_with_ns(ns, key)?;
        self.db.delete(&key)?;
        Ok(())
    }

    /// List all cache entries as JSON.
    pub fn list_json(&self) -> Result<Vec<JsonEntry>, Error> {
        let mut out = Vec::new();

        for (key, value) in self.db.iterator(rocksdb::IteratorMode::Start) {
            let key: json::Value = match cbor::from_slice(&*key) {
                Ok(key) => key,
                // key is malformed.
                Err(_) => continue,
            };

            let stored = match cbor::from_slice(&*value) {
                Ok(storage) => storage,
                // something weird stored in there.
                Err(_) => continue,
            };

            out.push(JsonEntry { key, stored });
        }

        Ok(out)
    }

    /// Clean up stale entries.
    ///
    /// This could be called periodically if you want to reclaim space.
    fn cleanup(&self) -> Result<(), Error> {
        let now = Utc::now();

        for (key, value) in self.db.iterator(rocksdb::IteratorMode::Start) {
            let entry: PartialStoredEntry = match cbor::from_slice(&*value) {
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

                    // delete key since it's invalid.
                    self.db.delete(key)?;
                    continue;
                }
            };

            if entry.is_expired(now) {
                self.db.delete(key)?;
            }
        }

        Ok(())
    }

    /// Create a namespaced cache.
    ///
    /// The namespace must be unique to avoid conflicts.
    pub fn namespaced(&self, ns: impl AsRef<str>) -> Self {
        Self {
            ns: Some(Arc::new(ns.as_ref().to_string())),
            db: self.db.clone(),
        }
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
    #[inline(always)]
    fn inner_insert<T>(&self, key: &Vec<u8>, age: Duration, value: T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let expires_at = Utc::now() + age;

        let value = match cbor::to_vec(&StoredEntry { expires_at, value }) {
            Ok(value) => value,
            Err(e) => {
                log::trace!("store:{} *errored*", KeyFormat(key));
                return Err(e.into());
            }
        };

        log::trace!("store:{}", KeyFormat(key));
        self.db.put(key, value)?;
        Ok(())
    }

    /// Test an entry from the cache.
    pub fn test<K>(&self, key: K) -> Result<EntryRef<'_, ()>, Error>
    where
        K: serde::Serialize,
    {
        let key = self.key(&key)?;
        self.inner_test(key)
    }

    /// Load an entry from the cache.
    #[inline(always)]
    fn inner_test(&self, key: Vec<u8>) -> Result<EntryRef<'_, ()>, Error> {
        let value = match self.db.get(&key)? {
            Some(value) => value,
            None => {
                log::trace!("test:{} -> null (missing)", KeyFormat(&key));
                return Ok(EntryRef::missing(self, key));
            }
        };

        let storage: PartialStoredEntry = match cbor::from_slice(&value) {
            Ok(value) => value,
            Err(e) => {
                if log::log_enabled!(log::Level::Trace) {
                    log::warn!(
                        "{}: failed to deserialize: {}: {}",
                        KeyFormat(&key),
                        e,
                        KeyFormat(&value)
                    );
                } else {
                    log::warn!("{}: failed to deserialize: {}", KeyFormat(&key), e);
                }

                log::trace!("test:{} -> null (deserialize error)", KeyFormat(&key));
                return Ok(EntryRef::missing(self, key));
            }
        };

        if storage.is_expired(Utc::now()) {
            log::trace!("test:{} -> null (expired)", KeyFormat(&key));
            return Ok(EntryRef::expired(self, key, storage.into_stored_entry()));
        }

        log::trace!("test:{} -> *value*", KeyFormat(&key));
        Ok(EntryRef::fresh(self, key, storage.into_stored_entry()))
    }

    /// Load an entry from the cache.
    pub fn get<K, T>(&self, key: K) -> Result<EntryRef<'_, T>, Error>
    where
        K: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let key = self.key(&key)?;
        self.inner_get(key)
    }

    /// Load an entry from the cache.
    #[inline(always)]
    fn inner_get<T>(&self, key: Vec<u8>) -> Result<EntryRef<'_, T>, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let value = match self.db.get(&key)? {
            Some(value) => value,
            None => {
                log::trace!("load:{} -> null (missing)", KeyFormat(&key));
                return Ok(EntryRef::missing(self, key));
            }
        };

        let storage: StoredEntry<T> = match cbor::from_slice(&value) {
            Ok(value) => value,
            Err(e) => {
                if log::log_enabled!(log::Level::Trace) {
                    log::warn!(
                        "{}: failed to deserialize: {}: {}",
                        KeyFormat(&key),
                        e,
                        KeyFormat(&value)
                    );
                } else {
                    log::warn!("{}: failed to deserialize: {}", KeyFormat(&key), e);
                }

                log::trace!("load:{} -> null (deserialize error)", KeyFormat(&key));
                return Ok(EntryRef::missing(self, key));
            }
        };

        if storage.is_expired(Utc::now()) {
            log::trace!("load:{} -> null (expired)", KeyFormat(&key));
            return Ok(EntryRef::expired(self, key, storage));
        }

        log::trace!("load:{} -> *value*", KeyFormat(&key));
        Ok(EntryRef::fresh(self, key, storage))
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
        let key = match self.get(key)? {
            EntryRef { key, state, .. } => match state {
                State::Fresh(e) => return Ok(e.value),
                State::Expired(..) | State::Missing => key,
            },
        };

        let output = future.await?;
        self.inner_insert(&key, age.as_chrono(), output.clone())?;
        Ok(output)
    }

    /// Helper to serialize the key with the default namespace.
    fn key<T>(&self, key: T) -> Result<Vec<u8>, Error>
    where
        T: serde::Serialize,
    {
        self.key_with_ns(self.ns.as_ref().map(|ns| ns.as_str()), key)
    }

    /// Helper to serialize the key with a specific namespace.
    fn key_with_ns<T>(&self, ns: Option<&str>, key: T) -> Result<Vec<u8>, Error>
    where
        T: serde::Serialize,
    {
        let key = Key(ns, key);
        return cbor::to_vec(&key).map_err(Into::into);

        #[derive(serde::Serialize)]
        struct Key<'a, T>(Option<&'a str>, T);
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
