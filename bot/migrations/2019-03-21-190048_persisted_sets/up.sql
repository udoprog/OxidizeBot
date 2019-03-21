CREATE TABLE set_values (
    channel VARCHAR NOT NULL,
    kind VARCHAR NOT NULL,
    value VARCHAR NOT NULL,
    PRIMARY KEY (channel, kind, value)
);