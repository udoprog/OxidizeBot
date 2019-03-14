use crate::{db, template};
use failure::{format_err, ResultExt as _};
use hashbrown::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

/// The backend of a words store.
pub trait Backend: Clone + Send + Sync {
    /// List all counters in backend.
    fn list(&self) -> Result<Vec<db::Counter>, failure::Error>;

    /// Insert or update an existing counter.
    fn edit(&self, channel: &str, name: &str, text: &str) -> Result<(), failure::Error>;

    /// Delete the given counter from the backend.
    fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error>;

    /// Increment the counter.
    /// Returns `true` if the counter existed and was incremented. `false` otherwise.
    fn increment(&self, channel: &str, name: &str) -> Result<bool, failure::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug)]
pub struct Counters<B>
where
    B: Backend,
{
    inner: RwLock<HashMap<Key, Arc<Counter>>>,
    backend: B,
}

impl<B> Counters<B>
where
    B: Backend,
{
    /// Construct a new counters store with a backend.
    pub fn new(backend: B) -> Counters<B> {
        Counters {
            inner: RwLock::new(Default::default()),
            backend,
        }
    }

    /// Load all counters from the backend.
    pub fn load_from_backend(&mut self) -> Result<(), failure::Error> {
        let mut inner = self.inner.write().expect("lock poisoned");

        for counter in self.backend.list()? {
            let key = Key::new(counter.channel.as_str(), counter.name.as_str());

            let template =
                template::Template::compile(counter.text.as_str()).with_context(|_| {
                    format_err!("failed to compile template `{:?}` from backend", key)
                })?;

            inner.insert(
                key.clone(),
                Arc::new(Counter {
                    key,
                    // TODO: use other integer atomics when available.
                    count: AtomicUsize::new(counter.count as usize),
                    template,
                }),
            );
        }

        Ok(())
    }

    /// Insert a word into the bad words list.
    pub fn edit(&self, channel: &str, name: &str, text: &str) -> Result<(), failure::Error> {
        let key = Key::new(channel, name);

        let template = template::Template::compile(text)?;
        self.backend
            .edit(key.channel.as_str(), key.name.as_str(), text)?;

        let mut inner = self.inner.write().expect("lock poisoned");
        let count = inner.get(&key).map(|c| c.count()).unwrap_or(0);

        inner.insert(
            key.clone(),
            Arc::new(Counter {
                key,
                count: AtomicUsize::new(count as usize),
                template,
            }),
        );

        Ok(())
    }

    /// Remove a word from the bad words list.
    pub fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        let key = Key::new(channel, name);

        if !self
            .backend
            .delete(key.channel.as_str(), key.name.as_str())?
        {
            return Ok(false);
        }

        let mut inner = self.inner.write().expect("lock poisoned");
        inner.remove(&key);
        Ok(true)
    }

    /// Test the given word.
    pub fn get<'a>(&'a self, channel: &str, name: &str) -> Option<Arc<Counter>> {
        let key = Key::new(channel, name);

        let inner = self.inner.read().expect("lock poisoned");

        if let Some(counter) = inner.get(&key) {
            return Some(Arc::clone(counter));
        }

        None
    }

    /// Get a list of all counters.
    pub fn list(&self, channel: &str) -> Vec<Arc<Counter>> {
        let inner = self.inner.read().expect("lock poisoned");

        let mut out = Vec::new();

        for c in inner.values() {
            if c.key.channel != channel {
                continue;
            }

            out.push(Arc::clone(c));
        }

        out
    }

    /// Increment the specified counter.
    pub fn increment(&self, counter: &Counter) -> Result<(), failure::Error> {
        self.backend
            .increment(counter.key.channel.as_str(), counter.key.name.as_str())?;
        counter.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Counter {
    pub key: Key,
    count: AtomicUsize,
    pub template: template::Template,
}

impl Counter {
    /// Get the currenct count.
    pub fn count(&self) -> i32 {
        self.count.load(Ordering::SeqCst) as i32
    }
}

impl Counter {
    /// Render the given counter.
    pub fn render<T>(&self, data: &T) -> Result<String, failure::Error>
    where
        T: serde::Serialize,
    {
        Ok(self.template.render_to_string(data)?)
    }
}
