CREATE TABLE cache (
  -- entry key.
  key VARCHAR NOT NULL,
  -- time it was added.
  expires_at TIMESTAMP NOT NULL,
  -- value in the cache.
  value VARCHAR NOT NULL,

  PRIMARY KEY(key)
);

CREATE INDEX idx_cache_expires_at ON cache(expires_at);