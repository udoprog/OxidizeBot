use crate::db;
use diesel::prelude::*;
use failure::{bail, Error};
use hashbrown::{HashMap, HashSet};
use parking_lot::RwLock;
use std::{fmt, sync::Arc};

const SCHEMA: &'static [u8] = include_bytes!("scopes.yaml");

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Schema {
    scopes: HashMap<Scope, ScopeData>,
}

impl Schema {
    /// Load schema from the given set of bytes.
    pub fn load_static() -> Result<Schema, failure::Error> {
        Ok(serde_yaml::from_slice(SCHEMA)?)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeInfo {
    scope: Scope,
    #[serde(flatten)]
    data: ScopeData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeData {
    /// Documentation for this scope.
    pub doc: String,
    /// Version of the schema.
    /// A change in version will cause the default allows to be applied.
    pub version: String,
    /// Default allows for the scope.
    pub allow: Vec<Role>,
}

/// A container for scopes and their allows.
#[derive(Clone)]
pub struct Auth {
    db: db::Database,
    /// Schema for every corresponding scope.
    pub schema: Arc<Schema>,
    /// Assignments.
    pub allows: Arc<RwLock<HashSet<(Scope, Role)>>>,
}

impl Auth {
    pub fn new(db: db::Database, schema: Schema) -> Result<Self, Error> {
        use db::schema::scope_allows::dsl;

        let allows = dsl::scope_allows
            .select((dsl::scope, dsl::role))
            .load::<(Scope, Role)>(&*db.pool.lock())?
            .into_iter()
            .collect::<HashSet<_>>();

        let scopes = Self {
            db,
            schema: Arc::new(schema),
            allows: Arc::new(RwLock::new(allows)),
        };

        // perform default scope migrations based on scopes.yaml
        scopes.insert_default_allows()?;
        Ok(scopes)
    }

    /// Insert default allows for non-initialized scopes.
    fn insert_default_allows(&self) -> Result<(), Error> {
        use db::schema::scope_inits::dsl;

        let allows = dsl::scope_inits
            .select((dsl::scope, dsl::version))
            .load::<(Scope, String)>(&*self.db.pool.lock())?
            .into_iter()
            .collect::<HashMap<Scope, String>>();

        let mut to_insert = Vec::new();

        for (key, data) in &self.schema.scopes {
            let version = match allows.get(key) {
                Some(version) => version,
                None => {
                    to_insert.push((*key, data));
                    continue;
                }
            };

            if data.version != *version {
                to_insert.push((*key, data));
            }
        }

        for (key, data) in to_insert {
            for allow in &data.allow {
                self.insert(key, *allow)?;
            }

            diesel::insert_into(dsl::scope_inits)
                .values((dsl::scope.eq(key), dsl::version.eq(&data.version)))
                .execute(&*self.db.pool.lock())?;
        }

        Ok(())
    }

    /// Insert an assignment.
    pub fn insert(&self, scope: Scope, role: Role) -> Result<(), Error> {
        use db::schema::scope_allows::dsl;

        diesel::insert_into(dsl::scope_allows)
            .values((dsl::scope.eq(scope), dsl::role.eq(role)))
            .execute(&*self.db.pool.lock())?;

        self.allows.write().insert((scope, role));
        Ok(())
    }

    /// Delete an assignment.
    pub fn delete(&self, scope: Scope, role: Role) -> Result<(), Error> {
        use db::schema::scope_allows::dsl;

        if self.allows.write().remove(&(scope, role)) {
            let _ = diesel::delete(
                dsl::scope_allows.filter(dsl::scope.eq(scope).and(dsl::role.eq(role))),
            )
            .execute(&*self.db.pool.lock())?;
        }

        Ok(())
    }

    /// Test if the given assignment exists.
    pub fn test(&self, scope: Scope, role: Role) -> bool {
        self.allows.read().contains(&(scope, role))
    }

    /// Test if the given assignment exists.
    pub fn test_any(&self, scope: Scope, roles: impl IntoIterator<Item = Role>) -> bool {
        let allows = self.allows.read();
        roles.into_iter().any(|r| allows.contains(&(scope, r)))
    }

    /// Get a list of scopes and extra information associated with them.
    pub fn scopes(&self) -> Vec<ScopeInfo> {
        let mut out = Vec::new();

        for scope in Scope::list() {
            let data = match self.schema.scopes.get(&scope) {
                Some(data) => data,
                None => continue,
            };

            out.push(ScopeInfo {
                scope,
                data: data.clone(),
            });
        }

        out
    }

    /// Get a list of roles.
    pub fn roles(&self) -> Vec<Role> {
        Role::list()
    }

    /// Get a list of all allows.
    pub fn list(&self) -> Vec<(Scope, Role)> {
        self.allows.read().iter().cloned().collect()
    }
}

macro_rules! scopes {
    ($(($variant:ident, $scope:expr),)*) => {
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        serde::Serialize,
        serde::Deserialize,
        diesel::FromSqlRow,
        diesel::AsExpression,
    )]
    #[sql_type = "diesel::sql_types::Text"]
    pub enum Scope {
        $(#[serde(rename = $scope)] $variant,)*
    }

    impl Scope {
        /// Get a list of all scopes.
        pub fn list() -> Vec<Scope> {
            vec![
                $(Scope::$variant,)*
            ]
        }
    }

    impl fmt::Display for Scope {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                $(Scope::$variant => $scope.fmt(fmt),)*
            }
        }
    }

    impl std::str::FromStr for Scope {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                $($scope => Ok(Scope::$variant),)*
                other => bail!("bad scope: {}", other),
            }
        }
    }

    impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for Scope
    where
        DB: diesel::backend::Backend,
        String: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
    {
        fn to_sql<W>(&self, out: &mut diesel::serialize::Output<W, DB>) -> diesel::serialize::Result
        where
            W: std::io::Write,
        {
            self.to_string().to_sql(out)
        }
    }

    impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for Scope
    where
        DB: diesel::backend::Backend,
        String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
    {
        fn from_sql(bytes: Option<&DB::RawValue>) -> diesel::deserialize::Result<Self> {
            let s = String::from_sql(bytes)?;
            Ok(str::parse(&s)?)
        }
    }
    }
}

macro_rules! roles {
    ($(($variant:ident, $role:expr),)*) => {
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        serde::Serialize,
        serde::Deserialize,
        diesel::FromSqlRow,
        diesel::AsExpression,
    )]
    #[sql_type = "diesel::sql_types::Text"]
    pub enum Role {
        $(#[serde(rename = $role)] $variant,)*
    }

    impl Role {
        /// Get a list of all roles.
        pub fn list() -> Vec<Role> {
            vec![
                $(Role::$variant,)*
            ]
        }
    }

    impl fmt::Display for Role {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                $(Role::$variant => $role.fmt(fmt),)*
            }
        }
    }

    impl std::str::FromStr for Role {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                $($role => Ok(Role::$variant),)*
                other => bail!("bad role: {}", other),
            }
        }
    }

    impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for Role
    where
        DB: diesel::backend::Backend,
        String: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
    {
        fn to_sql<W>(&self, out: &mut diesel::serialize::Output<W, DB>) -> diesel::serialize::Result
        where
            W: std::io::Write,
        {
            self.to_string().to_sql(out)
        }
    }

    impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for Role
    where
        DB: diesel::backend::Backend,
        String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
    {
        fn from_sql(bytes: Option<&DB::RawValue>) -> diesel::deserialize::Result<Self> {
            let s = String::from_sql(bytes)?;
            Ok(str::parse(&s)?)
        }
    }
    }
}

scopes! {
    (PlayerDetachDetach, "player/attach-detach"),
}

roles! {
    (Streamer, "@streamer"),
    (Moderator, "@moderator"),
    (Subscriber, "@subscriber"),
    (Other, "@other"),
}
