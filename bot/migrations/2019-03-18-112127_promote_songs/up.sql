ALTER TABLE songs ADD COLUMN promoted_at TIMESTAMP DEFAULT NULL;
ALTER TABLE songs ADD COLUMN promoted_by VARCHAR DEFAULT NULL;

CREATE INDEX songs_deleted_added_at ON songs (deleted, track_id);