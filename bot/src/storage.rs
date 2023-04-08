use anyhow::Result;
use std::path::Path;

pub(crate) use futures_cache::{sled, Cache};

pub(crate) struct Storage {
    db: sled::Db,
}

impl Storage {
    /// Open the given storage location.
    pub(crate) fn open(path: &Path) -> Result<Storage> {
        let db = sled::open(path.join("sled.34"))?;
        Ok(Storage { db })
    }

    /// Access the cache abstraction of your storage.
    pub(crate) fn cache(&self) -> Result<Cache> {
        Ok(Cache::load(self.db.open_tree("cache")?)?)
    }
}
