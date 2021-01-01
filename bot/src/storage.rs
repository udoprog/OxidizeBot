use anyhow::Result;
use std::path::Path;

pub use futures_cache::{sled, Cache};

pub struct Storage {
    db: sled::Db,
}

impl Storage {
    /// Open the given storage location.
    pub fn open(path: &Path) -> Result<Storage> {
        let db = sled::open(path.join("sled.34"))?;
        Ok(Storage { db })
    }

    /// Access the cache abstraction of your storage.
    pub fn cache(&self) -> Result<Cache> {
        Ok(Cache::load(self.db.open_tree("cache")?)?)
    }
}
