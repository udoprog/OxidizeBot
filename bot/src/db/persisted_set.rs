use crate::db;
use failure::{format_err, ResultExt as _};
use hashbrown::{HashMap, HashSet};
use std::{
    cmp, error, fmt, hash, marker, str,
    sync::{Arc, RwLock},
};

/// The backend of a words store.
pub trait Backend: Clone + Send + Sync {
    /// List all commands in backend.
    fn list(&self, kind: &str) -> Result<Vec<db::models::SetValue>, failure::Error>;

    /// Insert a url into the whitelist.
    fn insert(&self, channel: &str, kind: &str, value: String) -> Result<(), failure::Error>;

    /// Delete the given command from the backend.
    fn remove(&self, channel: &str, kind: &str, value: String) -> Result<bool, failure::Error>;
}

#[derive(Debug, Clone)]
pub struct PersistedSet<B, T>
where
    B: Backend,
    T: hash::Hash + cmp::Eq,
{
    inner: Arc<RwLock<HashMap<String, HashSet<T>>>>,
    kind: &'static str,
    backend: B,
    marker: marker::PhantomData<T>,
}

impl<B, T> PersistedSet<B, T>
where
    B: Backend,
    T: Clone + str::FromStr + hash::Hash + cmp::Eq + fmt::Display,
    <T as str::FromStr>::Err: error::Error + Send + Sync + 'static,
{
    /// Construct a new commands store with a backend.
    pub fn load(backend: B, kind: &'static str) -> Result<PersistedSet<B, T>, failure::Error> {
        let mut inner = HashMap::<String, HashSet<T>>::new();

        for v in backend.list(kind)? {
            let value = str::parse::<T>(&v.value)
                .with_context(|_| format_err!("failed to deserialize {:?}", v))?;
            inner.entry(v.channel).or_default().insert(value);
        }

        Ok(PersistedSet {
            inner: Arc::new(RwLock::new(inner)),
            kind,
            backend,
            marker: marker::PhantomData,
        })
    }

    /// Load all commands from the backend.
    pub fn load_from_backend(&mut self) -> Result<(), failure::Error> {
        Ok(())
    }

    /// Insert a word into the bad words list.
    pub fn insert(&self, channel: &str, value: T) -> Result<(), failure::Error> {
        let mut inner = self.inner.write().expect("lock poisoned");
        let values = inner.entry(channel.to_string()).or_default();

        if !values.contains(&value) {
            self.backend.insert(channel, self.kind, value.to_string())?;
            values.insert(value);
        }

        Ok(())
    }

    /// Remove the given value from the container.
    pub fn delete(&self, channel: &str, value: &T) -> Result<bool, failure::Error> {
        use hashbrown::hash_map;
        let mut inner = self.inner.write().expect("lock poisoned");

        if let hash_map::Entry::Occupied(mut e) = inner.entry(channel.to_string()) {
            if !e.get_mut().remove(value) {
                return Ok(false);
            }

            let value = value.to_string();
            self.backend.remove(channel, self.kind, value)?;
        }

        Ok(true)
    }

    /// Test the given word.
    pub fn contains<'a>(&'a self, channel: &str, value: &T) -> bool {
        let inner = self.inner.read().expect("lock poisoned");

        if let Some(values) = inner.get(channel) {
            return values.contains(value);
        }

        false
    }

    /// Get a list of all commands.
    pub fn list(&self, channel: &str) -> Vec<T> {
        let inner = self.inner.read().expect("lock poisoned");

        let mut out = Vec::new();

        if let Some(values) = inner.get(channel) {
            out.extend(values.iter().cloned());
        }

        out
    }
}
