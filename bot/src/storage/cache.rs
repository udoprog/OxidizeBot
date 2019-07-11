use crate::{prelude::*, utils};
use chrono::{DateTime, Duration, Utc};
use failure::Error;
use hex::ToHex as _;
use serde_cbor as cbor;
use serde_json as json;
use std::{fmt, sync::Arc};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Entry<T> {
    expires_at: DateTime<Utc>,
    value: T,
}

impl<T> Entry<T> {
    /// Test if entry is expired.
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at < now
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PartialEntry {
    expires_at: DateTime<Utc>,
}

impl PartialEntry {
    /// Test if entry is expired.
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at < now
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

    /// Clean up stale entries.
    ///
    /// This could be called periodically if you want to reclaim space.
    fn cleanup(&self) -> Result<(), Error> {
        let now = Utc::now();

        for (key, value) in self.db.iterator(rocksdb::IteratorMode::Start) {
            let entry: PartialEntry = match cbor::from_slice(&*value) {
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

        let value = match cbor::to_vec(&Entry { expires_at, value }) {
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

    /// Load an entry from the cache.
    pub fn get<K, T>(&self, key: K) -> Result<Option<T>, Error>
    where
        K: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let key = self.key(&key)?;
        let (delete, result) = self.inner_get(&key)?;

        if delete {
            self.db.delete(&key)?;
        }

        Ok(result)
    }

    /// Load an entry from the cache.
    #[inline(always)]
    fn inner_get<T>(&self, key: &Vec<u8>) -> Result<(bool, Option<T>), Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let value = match self.db.get(key)? {
            Some(value) => value,
            None => {
                log::trace!("load:{} -> null (missing)", KeyFormat(key));
                return Ok((false, None));
            }
        };

        let entry: Entry<T> = match cbor::from_slice(&value) {
            Ok(value) => value,
            Err(e) => {
                if log::log_enabled!(log::Level::Trace) {
                    log::warn!(
                        "{}: failed to deserialize: {}: {}",
                        KeyFormat(key),
                        e,
                        KeyFormat(&value)
                    );
                } else {
                    log::warn!("{}: failed to deserialize: {}", KeyFormat(key), e);
                }

                log::trace!("load:{} -> null (deserialize error)", KeyFormat(key));
                return Ok((false, None));
            }
        };

        if entry.is_expired(Utc::now()) {
            log::trace!("load:{} -> null (expired)", KeyFormat(key));
            return Ok((true, None));
        }

        log::trace!("load:{} -> *value*", KeyFormat(key));
        Ok((false, Some(entry.value)))
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

        let (_, result) = self.inner_get(&key)?;

        if let Some(output) = result {
            return Ok(output);
        }

        let output = future.await?;
        self.inner_insert(&key, age.as_chrono(), output.clone())?;
        Ok(output)
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
