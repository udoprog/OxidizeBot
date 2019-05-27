use crate::db;
use diesel::prelude::*;
use failure::{Error, ResultExt as _};
use hashbrown::{HashMap, HashSet};
use parking_lot::RwLock;
use std::{fmt, sync::Arc};

const SCHEMA: &'static [u8] = include_bytes!("auth.yaml");

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Schema {
    roles: HashMap<Role, RoleData>,
    scopes: HashMap<Scope, ScopeData>,
}

impl Schema {
    /// Load schema from the given set of bytes.
    pub fn load_static() -> Result<Schema, failure::Error> {
        Ok(serde_yaml::from_slice(SCHEMA).context("failed to load auth.yaml")?)
    }
}

/// A container for scopes and their grants.
#[derive(Clone)]
pub struct Auth {
    db: db::Database,
    /// Schema for every corresponding scope.
    pub schema: Arc<Schema>,
    /// Assignments.
    pub grants: Arc<RwLock<HashSet<(Scope, Role)>>>,
}

impl Auth {
    pub fn new(db: db::Database, schema: Schema) -> Result<Self, Error> {
        use db::schema::grants::dsl;

        let grants = dsl::grants
            .select((dsl::scope, dsl::role))
            .load::<(Scope, Role)>(&*db.pool.lock())?
            .into_iter()
            .collect::<HashSet<_>>();

        let auth = Self {
            db,
            schema: Arc::new(schema),
            grants: Arc::new(RwLock::new(grants)),
        };

        // perform default initialization based on auth.yaml
        auth.insert_default_grants()?;
        Ok(auth)
    }

    /// Insert default grants for non-initialized grants.
    fn insert_default_grants(&self) -> Result<(), Error> {
        use db::schema::initialized_grants::dsl;

        let grants = dsl::initialized_grants
            .select((dsl::scope, dsl::version))
            .load::<(Scope, String)>(&*self.db.pool.lock())?
            .into_iter()
            .collect::<HashMap<Scope, String>>();

        let mut to_insert = Vec::new();

        for (key, data) in &self.schema.scopes {
            let version = match grants.get(key) {
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

            diesel::insert_into(dsl::initialized_grants)
                .values((dsl::scope.eq(key), dsl::version.eq(&data.version)))
                .execute(&*self.db.pool.lock())?;
        }

        Ok(())
    }

    /// Insert an assignment.
    pub fn insert(&self, scope: Scope, role: Role) -> Result<(), Error> {
        use db::schema::grants::dsl;

        diesel::insert_into(dsl::grants)
            .values((dsl::scope.eq(scope), dsl::role.eq(role)))
            .execute(&*self.db.pool.lock())?;

        self.grants.write().insert((scope, role));
        Ok(())
    }

    /// Delete an assignment.
    pub fn delete(&self, scope: Scope, role: Role) -> Result<(), Error> {
        use db::schema::grants::dsl;

        if self.grants.write().remove(&(scope, role)) {
            let _ =
                diesel::delete(dsl::grants.filter(dsl::scope.eq(scope).and(dsl::role.eq(role))))
                    .execute(&*self.db.pool.lock())?;
        }

        Ok(())
    }

    /// Test if the given assignment exists.
    pub fn test(&self, scope: Scope, role: Role) -> bool {
        self.grants.read().contains(&(scope, role))
    }

    /// Test if the given assignment exists.
    pub fn test_any(&self, scope: Scope, roles: impl IntoIterator<Item = Role>) -> bool {
        let grants = self.grants.read();
        roles.into_iter().any(|r| grants.contains(&(scope, r)))
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
    pub fn roles(&self) -> Vec<RoleInfo> {
        let mut out = Vec::new();

        for role in Role::list() {
            let data = match self.schema.roles.get(&role) {
                Some(data) => data,
                None => continue,
            };

            out.push(RoleInfo {
                role,
                data: data.clone(),
            });
        }

        out
    }

    /// Get a list of all grants.
    pub fn list(&self) -> Vec<(Scope, Role)> {
        self.grants.read().iter().cloned().collect()
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
        Unknown,
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
                Scope::Unknown => "unknown".fmt(fmt),
            }
        }
    }

    impl std::str::FromStr for Scope {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                $($scope => Ok(Scope::$variant),)*
                _ => Ok(Scope::Unknown),
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
        Unknown,
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
                Role::Unknown => "unknown".fmt(fmt),
            }
        }
    }

    impl std::str::FromStr for Role {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                $($role => Ok(Role::$variant),)*
                _ => Ok(Role::Unknown),
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

/// The risk of a given scope.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Risk {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "default", other)]
    Default,
}

impl Default for Risk {
    fn default() -> Self {
        Risk::Default
    }
}

scopes! {
    (PlayerDetachDetach, "player/attach-detach"),
    (CommandAdmin, "command/admin"),
    (CommandSong, "command/song"),
    (CommandSongYouTube, "command/song/youtube"),
    (CommandSongSpotify, "command/song/spotify"),
    (CommandSwearJar, "command/swearjar"),
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
    /// How risky is this scope to grant.
    /// High risk grants should be prompted with a warning before granted.
    #[serde(default)]
    pub risk: Risk,
    /// Version of the schema.
    /// A change in version will cause the default grants to be applied.
    pub version: String,
    /// Default grants for the scope.
    pub allow: Vec<Role>,
}

roles! {
    (Streamer, "@streamer"),
    (Moderator, "@moderator"),
    (Subscriber, "@subscriber"),
    (Everyone, "@everyone"),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleInfo {
    role: Role,
    #[serde(flatten)]
    data: RoleData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleData {
    /// Documentation for this role.
    pub doc: String,
}
