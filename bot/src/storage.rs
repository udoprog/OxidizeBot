use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

pub use futures_cache::{sled, Cache};

pub struct Storage {
    db: Arc<sled::Db>,
}

impl Storage {
    /// Open the given storage location.
    pub fn open(path: &Path) -> Result<Storage> {
        let db = sled::open(path.join("sled.31"))?;
        Ok(Storage { db: Arc::new(db) })
    }

    /// Access the cache abstraction of your storage.
    pub fn cache(&self) -> Result<Cache> {
        Ok(Cache::load(Arc::new(self.db.open_tree("cache")?))?)
    }
}
