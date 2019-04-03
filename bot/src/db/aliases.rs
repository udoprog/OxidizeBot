use crate::{db, template, utils};
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// The backend of a words store.
pub trait Backend: Clone + Send + Sync {
    /// List all commands in backend.
    fn list(&self) -> Result<Vec<db::models::Alias>, failure::Error>;

    /// Insert or update an existing alias.
    fn edit(&self, key: &Key, text: &str) -> Result<(), failure::Error>;

    /// Delete the given alias from the backend.
    fn delete(&self, key: &Key) -> Result<bool, failure::Error>;

    /// Rename the given alias.
    fn rename(&self, from_key: &Key, to_key: &Key) -> Result<bool, failure::Error>;
}

#[derive(Debug, Clone)]
pub struct Aliases<B>
where
    B: Backend,
{
    inner: Arc<RwLock<HashMap<Key, Arc<Alias>>>>,
    backend: B,
}

impl<B> Aliases<B>
where
    B: Backend,
{
    /// Construct a new commands store with a backend.
    pub fn load(backend: B) -> Result<Aliases<B>, failure::Error> {
        let mut inner = HashMap::new();

        for alias in backend.list()? {
            let key = Key::new(alias.channel.as_str(), alias.name.as_str());
            let template = template::Template::compile(alias.text)?;
            inner.insert(key.clone(), Arc::new(Alias { key, template }));
        }

        Ok(Aliases {
            inner: Arc::new(RwLock::new(inner)),
            backend,
        })
    }

    pub fn lookup<'a>(&self, channel: &str, it: utils::Words<'a>) -> Option<(&'a str, String)> {
        let it = it.into_iter();

        let inner = self.inner.read();

        for (key, alias) in inner.iter() {
            if key.channel != channel {
                continue;
            }

            if let Some((m, out)) = alias.matches(it.clone()) {
                return Some((m, out));
            }
        }

        None
    }

    /// Insert a word into the bad words list.
    pub fn edit(&self, channel: &str, name: &str, text: &str) -> Result<(), failure::Error> {
        let key = Key::new(channel, name);
        let template = template::Template::compile(text)?;
        self.backend.edit(&key, text)?;
        self.inner
            .write()
            .insert(key.clone(), Arc::new(Alias { key, template }));
        Ok(())
    }

    /// Remove alias.
    pub fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        let key = Key::new(channel, name);

        if !self.backend.delete(&key)? {
            return Ok(false);
        }

        self.inner.write().remove(&key);
        Ok(true)
    }

    /// Test the given word.
    pub fn get<'a>(&'a self, channel: &str, name: &str) -> Option<Arc<Alias>> {
        let key = Key::new(channel, name);

        let inner = self.inner.read();

        if let Some(alias) = inner.get(&key) {
            return Some(Arc::clone(alias));
        }

        None
    }

    /// Get a list of all commands.
    pub fn list(&self, channel: &str) -> Vec<Arc<Alias>> {
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

    /// Try to rename the alias.
    pub fn rename(&self, channel: &str, from: &str, to: &str) -> Result<(), super::RenameError> {
        let from_key = Key::new(channel, from);
        let to_key = Key::new(channel, to);

        let mut inner = self.inner.write();

        if inner.contains_key(&to_key) {
            return Err(super::RenameError::Conflict);
        }

        let alias = match inner.remove(&from_key) {
            Some(alias) => alias,
            None => return Err(super::RenameError::Missing),
        };

        let alias = Alias {
            key: to_key.clone(),
            template: alias.template.clone(),
        };

        match self.backend.rename(&from_key, &to_key) {
            Err(e) => {
                log::error!("failed to rename alias `{}` in database: {}", from, e);
            }
            Ok(false) => {
                log::warn!("alias {} not renamed in database", from);
            }
            Ok(true) => (),
        }

        inner.insert(to_key, Arc::new(alias));
        Ok(())
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
pub struct Alias {
    pub key: Key,
    pub template: template::Template,
}

impl Alias {
    /// Test if the given input matches and return the corresonding replacement if it does.
    pub fn matches<'a>(&self, mut it: utils::Words<'a>) -> Option<(&'a str, String)> {
        match it.next() {
            Some(value) if value.to_lowercase() == self.key.name => {
                let data = Data { rest: it.rest() };

                match self.template.render_to_string(&data) {
                    Ok(s) => return Some((value, s)),
                    Err(e) => {
                        log::error!("failed to render alias: {}", e);
                    }
                }
            }
            _ => {}
        }

        return None;

        #[derive(serde::Serialize)]
        struct Data<'a> {
            rest: &'a str,
        }
    }
}
