CREATE TABLE songs2 (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    track_id VARCHAR NOT NULL,
    added_at TIMESTAMP NOT NULL,
    user VARCHAR,
    promoted_at TIMESTAMP DEFAULT NULL,
    promoted_by VARCHAR DEFAULT NULL
);

INSERT INTO songs2 (id, deleted, track_id, added_at, user, promoted_at, promoted_by) 
SELECT id, (deleted OR played) AS deleted, track_id, added_at, user, promoted_at, promoted_by FROM songs;

DROP TABLE songs;
ALTER TABLE songs2 RENAME TO songs;
