CREATE TABLE songs (
    channel VARCHAR NOT NULL,
    track_id VARCHAR NOT NULL,
    added_at TIMESTAMP NOT NULL,
    user VARCHAR,

    PRIMARY KEY (channel, track_id)
);
