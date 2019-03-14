CREATE TABLE balances (
    channel VARCHAR NOT NULL,
    user VARCHAR NOT NULL,
    amount INTEGER,
    PRIMARY KEY (channel, user)
);

CREATE TABLE commands (
    channel VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (channel, name)
);