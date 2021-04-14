ALTER TABLE songs
ADD COLUMN played BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE songs
SET
    played = deleted,
    deleted = FALSE;
