CREATE TABLE aliases (
    channel VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (channel, name)
);