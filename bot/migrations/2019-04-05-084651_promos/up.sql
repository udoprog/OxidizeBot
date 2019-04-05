CREATE TABLE promotions (
    channel VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    frequency INTEGER NOT NULL,
    text TEXT NOT NULL,
    promoted_at TIMESTAMP,
    PRIMARY KEY (channel, name)
);