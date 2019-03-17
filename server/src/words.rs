use crate::{db, template};
use hashbrown::HashMap;
use std::{
    fs::File,
    path::Path,
    sync::{Arc, RwLock, RwLockReadGuard},
};

/// Tokenize the given word.
pub fn tokenize(word: &str) -> String {
    let word = word.to_lowercase();
    inflector::string::singularize::to_singular(&word)
}

#[derive(Debug, Default)]
struct Inner {
    hashed: HashMap<eudex::Hash, Arc<Word>>,
    exact: HashMap<String, Arc<Word>>,
}

impl Inner {
    /// Insert a bad word.
    fn insert(&mut self, word: &str, why: Option<&str>) -> Result<(), failure::Error> {
        let word = Word {
            word: tokenize(word),
            why: why.map(template::Template::compile).transpose()?,
        };

        let word = Arc::new(word);
        self.hashed
            .insert(eudex::Hash::new(&word.word), Arc::clone(&word));
        self.exact.insert(word.word.to_string(), Arc::clone(&word));
        Ok(())
    }

    /// Insert a bad word.
    fn remove(&mut self, word: &str) {
        let word = tokenize(word);

        // TODO: there might be hash conflicts. Deal with them.
        self.hashed.remove(&eudex::Hash::new(&word));
        self.exact.remove(&word);
    }
}

/// The backend of a words store.
pub trait Backend: Clone + Send + Sync {
    /// List all words in backend.
    fn list(&self) -> Result<Vec<db::BadWord>, failure::Error>;

    /// Insert or update an existing word.
    fn edit(&self, word: &str, why: Option<&str>) -> Result<(), failure::Error>;

    /// Delete the given word from the backend.
    fn delete(&self, word: &str) -> Result<bool, failure::Error>;
}

#[derive(Debug, Clone)]
pub struct Words<B>
where
    B: Backend,
{
    inner: Arc<RwLock<Inner>>,
    backend: B,
}

impl<B> Words<B>
where
    B: Backend,
{
    /// Construct a new words store with a backend.
    pub fn new(backend: B) -> Words<B> {
        Words {
            inner: Arc::new(RwLock::new(Default::default())),
            backend,
        }
    }

    /// Load bad words configuration from a file.
    ///
    /// This will not store them in the database, and will prevent them from being deleted.
    pub fn load_from_path(&mut self, path: &Path) -> Result<(), failure::Error> {
        let config: Config = serde_yaml::from_reader(File::open(path)?)?;

        let mut inner = self.inner.write().expect("lock poisoned");

        for word in config.words {
            inner.insert(&word.word, word.why.as_ref().map(|s| s.as_str()))?;
        }

        Ok(())
    }

    /// Load all words from the backend.
    pub fn load_from_backend(&mut self) -> Result<(), failure::Error> {
        let mut inner = self.inner.write().expect("lock poisoned");

        for word in self.backend.list()? {
            inner.insert(&word.word, word.why.as_ref().map(|s| s.as_str()))?;
        }

        Ok(())
    }

    /// Insert a word into the bad words list.
    pub fn edit(&self, word: &str, why: Option<&str>) -> Result<(), failure::Error> {
        self.backend.edit(word, why)?;
        let mut inner = self.inner.write().expect("lock poisoned");
        inner.insert(word, why)?;
        Ok(())
    }

    /// Remove a word from the bad words list.
    pub fn delete(&self, word: &str) -> Result<bool, failure::Error> {
        if !self.backend.delete(word)? {
            return Ok(false);
        }

        let mut inner = self.inner.write().expect("lock poisoned");
        inner.remove(word);
        Ok(true)
    }

    /// Build a tester.
    pub fn tester(&self) -> Tester<'_> {
        let inner = self.inner.read().expect("lock poisoned");

        Tester { inner }
    }
}

/// A locked tester.
#[derive(Debug)]
pub struct Tester<'a> {
    inner: RwLockReadGuard<'a, Inner>,
}

impl Tester<'_> {
    /// Test the given word.
    pub fn test(&self, word: &str) -> Option<Arc<Word>> {
        let word = tokenize(word);

        if let Some(w) = self.inner.hashed.get(&eudex::Hash::new(&word)) {
            return Some(Arc::clone(w));
        }

        if let Some(w) = self.inner.exact.get(&word) {
            return Some(Arc::clone(w));
        }

        None
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    words: Vec<WordConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WordConfig {
    pub word: String,
    pub why: Option<String>,
}

#[derive(Debug)]
pub struct Word {
    pub word: String,
    pub why: Option<template::Template>,
}
