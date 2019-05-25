CREATE TABLE scopes (
    scope VARCHAR NOT NULL,
    role VARCHAR NOT NULL,
    PRIMARY KEY (scope, role)
);

CREATE INDEX scopes_role_scopes ON scopes(role, scope);
CREATE INDEX scopes_scope_role ON scopes(scope, role);