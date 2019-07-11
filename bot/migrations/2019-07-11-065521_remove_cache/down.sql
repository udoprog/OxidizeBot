CREATE TABLE cache (
  key VARCHAR NOT NULL,
  expires_at TIMESTAMP NOT NULL,
  value VARCHAR NOT NULL,
  PRIMARY KEY(key)
);

CREATE INDEX idx_cache_expires_at ON cache(expires_at);