use failure::Error;
use std::{path::Path, sync::Arc};

pub use futures_cache::{sled, Cache};

pub struct Storage {
    db: Arc<sled::Db>,
}

impl Storage {
    /// Open the given storage location.
    pub fn open(path: &Path) -> Result<Storage, Error> {
        let db = sled::Db::start_default(path.join("sled.24"))?;
        Ok(Storage { db: Arc::new(db) })
    }

    /// Access the cache abstraction of your storage.
    pub fn cache(&self) -> Result<Cache, Error> {
        Ok(Cache::load(self.db.open_tree("cache")?)?)
    }
}
