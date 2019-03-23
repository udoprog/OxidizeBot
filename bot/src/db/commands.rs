use crate::{db, template};
use failure::{format_err, ResultExt as _};
use hashbrown::HashMap;
use std::sync::{Arc, RwLock};

/// The backend of a words store.
pub trait Backend: Clone + Send + Sync {
    /// List all commands in backend.
    fn list(&self) -> Result<Vec<db::models::Command>, failure::Error>;

    /// Insert or update an existing command.
    fn edit(&self, channel: &str, word: &str, text: &str) -> Result<(), failure::Error>;

    /// Delete the given command from the backend.
    fn delete(&self, channel: &str, word: &str) -> Result<bool, failure::Error>;
}

#[derive(Debug, Clone)]
pub struct Commands<B>
where
    B: Backend,
{
    inner: Arc<RwLock<HashMap<Key, Arc<Command>>>>,
    backend: B,
}

impl<B> Commands<B>
where
    B: Backend,
{
    /// Construct a new commands store with a backend.
    pub fn load(backend: B) -> Result<Commands<B>, failure::Error> {
        let mut inner = HashMap::new();

        for command in backend.list()? {
            let template = template::Template::compile(&command.text).with_context(|_| {
                format_err!("failed to compile command `{:?}` from backend", command)
            })?;

            let key = Key::new(command.channel.as_str(), command.name.as_str());

            inner.insert(key.clone(), Arc::new(Command { key, template }));
        }

        Ok(Commands {
            inner: Arc::new(RwLock::new(inner)),
            backend,
        })
    }

    /// Insert a word into the bad words list.
    pub fn edit(&self, channel: &str, name: &str, command: &str) -> Result<(), failure::Error> {
        let key = Key::new(channel, name);

        let template = template::Template::compile(command)?;
        self.backend
            .edit(key.channel.as_str(), key.name.as_str(), command)?;

        let mut inner = self.inner.write().expect("lock poisoned");

        inner.insert(key.clone(), Arc::new(Command { key, template }));

        Ok(())
    }

    /// Remove command.
    pub fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        let key = Key::new(channel, name);

        if !self
            .backend
            .delete(key.channel.as_str(), key.name.as_str())?
        {
            return Ok(false);
        }

        self.inner.write().expect("lock poisoned").remove(&key);
        Ok(true)
    }

    /// Test the given word.
    pub fn get<'a>(&'a self, channel: &str, name: &str) -> Option<Arc<Command>> {
        let key = Key::new(channel, name);

        let inner = self.inner.read().expect("lock poisoned");

        if let Some(command) = inner.get(&key) {
            return Some(Arc::clone(command));
        }

        None
    }

    /// Get a list of all commands.
    pub fn list(&self, channel: &str) -> Vec<Arc<Command>> {
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
pub struct Command {
    pub key: Key,
    pub template: template::Template,
}

impl Command {
    /// Render the given command.
    pub fn render<T>(&self, data: &T) -> Result<String, failure::Error>
    where
        T: serde::Serialize,
    {
        Ok(self.template.render_to_string(data)?)
    }
}
