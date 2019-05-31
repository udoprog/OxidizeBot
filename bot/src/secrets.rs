//! Helper for storing and loading secrets.

use failure::ResultExt;
use hashbrown::HashMap;
use std::{fs::File, path::Path};

#[derive(Clone, serde::Deserialize)]
pub struct Secrets {
    #[serde(flatten)]
    secrets: HashMap<String, serde_yaml::Value>,
}

impl Secrets {
    /// Get an empty set of secrets.
    pub fn empty() -> Secrets {
        Secrets {
            secrets: Default::default(),
        }
    }

    /// Open the given file as secrets.
    pub fn open(path: impl AsRef<Path>) -> Result<Secrets, failure::Error> {
        let f = File::open(path)?;
        serde_yaml::from_reader(f).map_err(Into::into)
    }

    /// Load the given key of secrets.
    pub fn load<T>(&self, key: &str) -> Result<Option<T>, failure::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let value = match self.secrets.get(key) {
            Some(value) => value,
            None => return Ok(None),
        };

        let value = serde_yaml::from_value(value.clone())
            .with_context(|_| failure::format_err!("failed to deserialize secret `{}`", key))?;

        Ok(Some(value))
    }
}
