CREATE TABLE themes (
    channel VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    track_id TEXT NOT NULL,
    start INTEGER,
    end INTEGER,
    disabled BOOLEAN DEFAULT false,
    "group" TEXT,
    PRIMARY KEY (channel, name)
);