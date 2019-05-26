-- Scopes that have been initialized and their corresponding initialization
-- hash.
CREATE TABLE scope_inits (
    scope VARCHAR NOT NULL PRIMARY KEY,
    version VARCHAR NOT NULL
);

-- Scopes that have been allowed for specific roles.
CREATE TABLE scope_allows (
    scope VARCHAR NOT NULL,
    role VARCHAR NOT NULL,
    PRIMARY KEY (scope, role)
);