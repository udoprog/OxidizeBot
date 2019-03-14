CREATE TABLE after_streams (
    channel VARCHAR,
    added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    user TEXT,
    text TEXT,
    PRIMARY KEY (channel, added_at, user)
);
