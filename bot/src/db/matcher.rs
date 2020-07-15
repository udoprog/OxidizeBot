use crate::utils;
use anyhow::Error;
use std::collections::{hash_map, HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

/// Trait over something that has a matchable pattern.
pub trait Matchable {
    /// Get the key for the matchable element.
    fn key(&self) -> &Key;

    /// Get the pattern for the matchable element.
    fn pattern(&self) -> &Pattern;
}

pub struct Matcher<T>
where
    T: Matchable,
{
    /// All commands.
    all: HashMap<Key, Arc<T>>,
    /// Commands indexed by name.
    by_name: HashSet<Key>,
    /// Regular expression commands indexed by channel.
    by_channel_regex: HashMap<String, HashSet<Key>>,
}

impl<T> Matcher<T>
where
    T: Matchable,
{
    pub(crate) fn new() -> Self {
        Self {
            all: Default::default(),
            by_name: Default::default(),
            by_channel_regex: Default::default(),
        }
    }

    /// Test if we contain the given key.
    pub(crate) fn contains_key(&self, key: &Key) -> bool {
        self.all.contains_key(key)
    }

    /// Insert the given value.
    pub(crate) fn insert(&mut self, key: Key, value: Arc<T>) {
        match value.pattern() {
            Pattern::Name => {
                self.by_name.insert(key.clone());
            }
            Pattern::Regex { .. } => {
                self.by_channel_regex
                    .entry(key.channel.clone())
                    .or_default()
                    .insert(key.clone());
            }
        }

        self.all.insert(key, value);
    }

    /// Remove the given value.
    pub(crate) fn remove(&mut self, key: &Key) -> Option<Arc<T>> {
        if let Some(value) = self.all.remove(key) {
            match value.pattern() {
                Pattern::Name => {
                    self.by_name.remove(key);
                }
                Pattern::Regex { .. } => {
                    self.by_channel_regex
                        .entry(key.channel.clone())
                        .or_default()
                        .remove(&key);
                }
            }

            return Some(value);
        }

        None
    }

    /// Get an iterator over all the values.
    pub(crate) fn iter(&self) -> hash_map::Iter<'_, Key, Arc<T>> {
        self.all.iter()
    }

    /// Get an iterator over all the values.
    pub(crate) fn values(&self) -> hash_map::Values<'_, Key, Arc<T>> {
        self.all.values()
    }

    /// Get the underlying key.
    pub(crate) fn get(&self, key: &Key) -> Option<&Arc<T>> {
        self.all.get(key)
    }

    /// Modify the given element with the given pattern.
    /// Returns `true` if there was a value to modify. `false` otherwise.
    pub(crate) fn modify<F>(&mut self, key: Key, m: F) -> bool
    where
        T: Clone,
        F: FnOnce(&mut T),
    {
        let Self {
            all,
            by_channel_regex,
            by_name,
        } = self;

        let existing = match all.get_mut(&key) {
            Some(existing) => existing,
            None => return false,
        };

        let mut new = (**existing).clone();
        m(&mut new);

        // re-index in case pattern has changed.
        match new.pattern() {
            Pattern::Regex { .. } => {
                if let Pattern::Name = existing.pattern() {
                    by_name.remove(&key);

                    by_channel_regex
                        .entry(key.channel.clone())
                        .or_default()
                        .insert(key);
                }
            }
            Pattern::Name => {
                if let Pattern::Regex { .. } = existing.pattern() {
                    by_channel_regex
                        .entry(key.channel.clone())
                        .or_default()
                        .remove(&key);

                    by_name.insert(key);
                }
            }
        }

        *existing = Arc::new(new);
        true
    }

    /// Resolve the given command.
    pub fn resolve<'a>(
        &self,
        channel: &str,
        first: Option<&'a str>,
        it: &'a utils::Words,
    ) -> Option<(&Arc<T>, Captures<'a>)> {
        if let Some(first) = first {
            let key = Key::new(channel, first);

            if self.by_name.contains(&key) {
                if let Some(command) = self.get(&key) {
                    let captures = Captures::Prefix { rest: it.rest() };
                    return Some((command, captures));
                }
            }
        }

        if let Some(keys) = self.by_channel_regex.get(channel) {
            let full = it.string();

            for key in keys {
                if let Some(command) = self.get(key) {
                    if let Pattern::Regex { pattern } = command.pattern() {
                        if let Some(captures) = pattern.captures(full) {
                            let captures = Captures::Regex { captures };
                            return Some((command, captures));
                        }
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
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

impl fmt::Display for Key {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}/{}", self.channel, self.name)
    }
}

/// How to match the given value.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum Pattern {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "regex")]
    Regex {
        #[serde(serialize_with = "serialize_regex")]
        pattern: regex::Regex,
    },
}

impl Pattern {
    /// Construct a new pattern from a regular expression.
    pub fn regex(pattern: regex::Regex) -> Self {
        Self::Regex { pattern }
    }

    /// Convert a database pattern into a matchable pattern here.
    pub fn from_db(pattern: Option<impl AsRef<str>>) -> Result<Self, Error> {
        Ok(match pattern {
            Some(pattern) => Pattern::Regex {
                pattern: regex::Regex::new(pattern.as_ref())?,
            },
            None => Pattern::Name,
        })
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self::Name
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Name => "*name*".fmt(fmt),
            Pattern::Regex { pattern } => pattern.fmt(fmt),
        }
    }
}

/// Serialize a regular expression.
fn serialize_regex<S>(regex: &regex::Regex, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.collect_str(regex)
}

#[derive(Debug)]
pub enum Captures<'a> {
    Prefix { rest: &'a str },
    Regex { captures: regex::Captures<'a> },
}

impl<'a> Captures<'a> {
    /// Get the number of captures.
    fn len(&self) -> usize {
        match self {
            Self::Prefix { .. } => 1,
            Self::Regex { captures, .. } => captures.len(),
        }
    }
}

impl serde::Serialize for Captures<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap as _;

        let mut m = serializer.serialize_map(Some(self.len()))?;

        match self {
            Self::Prefix { rest } => {
                m.serialize_entry("rest", rest)?;
            }
            Self::Regex { captures, .. } => {
                for (i, g) in captures.iter().enumerate() {
                    m.serialize_entry(&i, &g.map(|m| m.as_str()))?;
                }
            }
        }

        m.end()
    }
}
