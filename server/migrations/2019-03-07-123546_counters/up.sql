CREATE TABLE counters (
    channel VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    count INTEGER,
    text VARCHAR,
    PRIMARY KEY (channel, name)
);
