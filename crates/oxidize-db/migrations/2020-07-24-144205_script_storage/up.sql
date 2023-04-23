CREATE TABLE script_keys (
    channel VARCHAR NOT NULL,
    key BLOB NOT NULL,
    value BLOB NOT NULL,
    PRIMARY KEY (channel, key)
);