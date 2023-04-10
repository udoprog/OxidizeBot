DROP INDEX songs_deleted_added_at;

CREATE TEMPORARY TABLE tmp_songs (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    track_id VARCHAR NOT NULL,
    added_at TIMESTAMP NOT NULL,
    user VARCHAR
);

INSERT INTO tmp_songs SELECT id, deleted, track_id, added_at, user FROM songs;
DROP TABLE songs;

CREATE TABLE songs (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    track_id VARCHAR NOT NULL,
    added_at TIMESTAMP NOT NULL,
    user VARCHAR
);

INSERT INTO songs SELECT id, deleted, track_id, added_at, user FROM tmp_songs;
DROP TABLE tmp_songs;