use crate::{db, template};
use parking_lot::{RwLock, RwLockReadGuard};
use std::{collections::HashMap, sync::Arc};

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
    fn insert(&mut self, word: &str, why: Option<&str>) -> Result<(), anyhow::Error> {
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
    fn list(&self) -> Result<Vec<db::models::BadWord>, anyhow::Error>;

    /// Insert or update an existing word.
    fn edit(&self, word: &str, why: Option<&str>) -> Result<(), anyhow::Error>;

    /// Delete the given word from the backend.
    fn delete(&self, word: &str) -> Result<bool, anyhow::Error>;
}

#[derive(Clone)]
pub struct Words {
    inner: Arc<RwLock<Inner>>,
    db: db::Database,
}

impl Words {
    /// Load all words from the backend.
    pub fn load(db: db::Database) -> Result<Words, anyhow::Error> {
        let mut inner = Inner::default();

        for word in db.list()? {
            inner.insert(&word.word, word.why.as_deref())?;
        }

        Ok(Words {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub fn edit(&self, word: &str, why: Option<&str>) -> Result<(), anyhow::Error> {
        self.db.edit(word, why)?;
        let mut inner = self.inner.write();
        inner.insert(word, why)?;
        Ok(())
    }

    /// Remove a word from the bad words list.
    pub fn delete(&self, word: &str) -> Result<bool, anyhow::Error> {
        if !self.db.delete(word)? {
            return Ok(false);
        }

        let mut inner = self.inner.write();
        inner.remove(word);
        Ok(true)
    }

    /// Build a tester.
    pub fn tester(&self) -> Tester<'_> {
        let inner = self.inner.read();

        Tester { inner }
    }
}

/// A locked tester.
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

#[derive(Debug)]
pub struct Word {
    pub word: String,
    pub why: Option<template::Template>,
}
