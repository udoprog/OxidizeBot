CREATE TABLE after_streams2 (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    channel VARCHAR,
    added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    user TEXT NOT NULL,
    text TEXT NOT NULL
);

INSERT INTO after_streams2 (channel, added_at, user, text) SELECT channel, added_at, user, text FROM after_streams;
DROP TABLE after_streams;
ALTER TABLE after_streams2 RENAME TO after_streams;