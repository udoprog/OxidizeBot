-- Grants that have been initialized and their corresponding initialization
-- hash.
CREATE TABLE initialized_grants (
    scope VARCHAR NOT NULL PRIMARY KEY,
    version VARCHAR NOT NULL
);

-- Grants that have been allowed for specific roles.
CREATE TABLE grants (
    scope VARCHAR NOT NULL,
    role VARCHAR NOT NULL,
    PRIMARY KEY (scope, role)
);