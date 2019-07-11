mod cache;

use failure::Error;
use std::{path::Path, sync::Arc};

pub use self::cache::Cache;

pub struct Storage {
    cache: Arc<rocksdb::DB>,
}

impl Storage {
    /// Open the given storage location.
    pub fn open(path: &Path) -> Result<Storage, Error> {
        let cache = create_db(&path.join("cache"))?;
        Ok(Storage { cache })
    }

    /// Access the cache abstraction of your storage.
    pub fn cache(&self) -> Result<Cache, Error> {
        Cache::load(self.cache.clone())
    }
}

/// Create a new environment based on the given path.
fn create_db(path: &Path) -> Result<Arc<rocksdb::DB>, Error> {
    if !path.is_dir() {
        std::fs::create_dir_all(path)?;
    }

    let mut options = rocksdb::Options::default();
    options.set_compression_type(rocksdb::DBCompressionType::Snappy);
    options.set_disable_auto_compactions(true);
    options.set_keep_log_file_num(16);
    options.create_if_missing(true);

    Ok(Arc::new(rocksdb::DB::open(&options, path)?))
}
