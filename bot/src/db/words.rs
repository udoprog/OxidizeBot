use crate::db;
use crate::template;
use diesel::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

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

#[derive(Clone)]
struct Database(db::Database);

impl Database {
    /// List all words in backend.
    async fn list(&self) -> Result<Vec<db::models::BadWord>, anyhow::Error> {
        use db::schema::bad_words::dsl;

        self.0
            .asyncify(move |c| Ok(dsl::bad_words.load::<db::models::BadWord>(c)?))
            .await
    }

    /// Insert or update an existing word.
    async fn edit(&self, word: &str, why: Option<&str>) -> Result<(), anyhow::Error> {
        use db::schema::bad_words::dsl;

        let word = word.to_string();
        let why = why.map(|w| w.to_string());

        self.0
            .asyncify(move |c| {
                let filter = dsl::bad_words.filter(dsl::word.eq(&word));
                let b = filter.clone().first::<db::models::BadWord>(c).optional()?;

                match b {
                    None => {
                        let bad_word = db::models::BadWord {
                            word,
                            why: why.map(|s| s.to_string()),
                        };

                        diesel::insert_into(dsl::bad_words)
                            .values(&bad_word)
                            .execute(c)?;
                    }
                    Some(_) => {
                        diesel::update(filter)
                            .set(why.map(|w| dsl::why.eq(w)))
                            .execute(c)?;
                    }
                }

                Ok(())
            })
            .await
    }

    /// Delete the given word from the backend.
    async fn delete(&self, word: &str) -> Result<bool, anyhow::Error> {
        use db::schema::bad_words::dsl;

        let word = word.to_string();

        self.0
            .asyncify(move |c| {
                let count =
                    diesel::delete(dsl::bad_words.filter(dsl::word.eq(&word))).execute(c)?;
                Ok(count == 1)
            })
            .await
    }
}

#[derive(Clone)]
pub struct Words {
    inner: Arc<RwLock<Inner>>,
    db: Database,
}

impl Words {
    /// Load all words from the backend.
    pub async fn load(db: db::Database) -> Result<Words, anyhow::Error> {
        let db = Database(db);
        let mut inner = Inner::default();

        for word in db.list().await? {
            inner.insert(&word.word, word.why.as_deref())?;
        }

        Ok(Words {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub async fn edit(&self, word: &str, why: Option<&str>) -> Result<(), anyhow::Error> {
        self.db.edit(word, why).await?;
        let mut inner = self.inner.write().await;
        inner.insert(word, why)?;
        Ok(())
    }

    /// Remove a word from the bad words list.
    pub async fn delete(&self, word: &str) -> Result<bool, anyhow::Error> {
        if !self.db.delete(word).await? {
            return Ok(false);
        }

        let mut inner = self.inner.write().await;
        inner.remove(word);
        Ok(true)
    }

    /// Build a tester.
    pub async fn tester(&self) -> Tester<'_> {
        let inner = self.inner.read().await;

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
