use crate::db;
use diesel::prelude::*;
use failure::{bail, Error};
use hashbrown::{HashMap, HashSet};
use parking_lot::RwLock;
use std::{fmt, sync::Arc};

const SCHEMA: &'static [u8] = include_bytes!("scopes.yaml");

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Schema {
    scopes: HashMap<String, ScopeData>,
}

impl Schema {
    /// Load schema from the given set of bytes.
    pub fn load_static() -> Result<Schema, failure::Error> {
        Ok(serde_yaml::from_slice(SCHEMA)?)
    }

    /// Lookup the given type by key.
    pub fn lookup(&self, key: &str) -> Option<ScopeData> {
        self.scopes.get(key).cloned()
    }

    /// Test if schema contains the given key.
    pub fn contains(&self, key: &str) -> bool {
        self.scopes.contains_key(key)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeData {
    /// Documentation for this scope.
    pub doc: String,
}

/// A container for scopes and their assignments.
#[derive(Clone)]
pub struct Scopes {
    db: db::Database,
    /// Schema for every corresponding scope.
    pub schema: Arc<Schema>,
    /// Assignments.
    pub assignments: Arc<RwLock<HashSet<(Scope, Role)>>>,
}

impl Scopes {
    pub fn new(db: db::Database, schema: Schema) -> Result<Self, Error> {
        use db::schema::scopes::dsl;

        let assignments = dsl::scopes
            .select((dsl::scope, dsl::role))
            .load::<(Scope, Role)>(&*db.pool.lock())?
            .into_iter()
            .collect::<HashSet<_>>();

        Ok(Self {
            db,
            schema: Arc::new(schema),
            assignments: Arc::new(RwLock::new(assignments)),
        })
    }

    /// Insert an assignment.
    pub fn insert(&self, scope: Scope, role: Role) -> Result<(), Error> {
        use db::schema::scopes::dsl;

        diesel::insert_into(dsl::scopes)
            .values((dsl::scope.eq(scope), dsl::role.eq(role)))
            .execute(&*self.db.pool.lock())?;

        self.assignments.write().insert((scope, role));
        Ok(())
    }

    /// Delete an assignment.
    pub fn delete(&self, scope: Scope, role: Role) -> Result<(), Error> {
        use db::schema::scopes::dsl;

        if self.assignments.write().remove(&(scope, role)) {
            let _ =
                diesel::delete(dsl::scopes.filter(dsl::scope.eq(scope).and(dsl::role.eq(role))))
                    .execute(&*self.db.pool.lock())?;
        }

        Ok(())
    }

    /// Test if the given assignment exists.
    pub fn test(&self, scope: Scope, role: Role) -> bool {
        self.assignments.read().contains(&(scope, role))
    }

    /// Get a list of all assignments.
    pub fn list(&self) -> Vec<(Scope, Role)> {
        self.assignments.read().iter().cloned().collect()
    }
}

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
    #[serde(rename = "player.detach")]
    PlayerDetach,
}

impl fmt::Display for Scope {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Scope::PlayerDetach => "player.detach".fmt(fmt),
        }
    }
}

impl std::str::FromStr for Scope {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "player.detach" => Ok(Scope::PlayerDetach),
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
    #[serde(rename = "@streamer")]
    Streamer,
    #[serde(rename = "@moderator")]
    Moderator,
    #[serde(rename = "@subscriber")]
    Subscriber,
    #[serde(rename = "@other")]
    Other,
}

impl fmt::Display for Role {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Role::Streamer => "@streamer".fmt(fmt),
            Role::Moderator => "@moderator".fmt(fmt),
            Role::Subscriber => "@subscriber".fmt(fmt),
            Role::Other => "@other".fmt(fmt),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "@streamer" => Ok(Role::Streamer),
            "@moderator" => Ok(Role::Moderator),
            "@subscriber" => Ok(Role::Subscriber),
            "@other" => Ok(Role::Other),
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
