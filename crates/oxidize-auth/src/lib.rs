use std::collections::{HashMap, HashSet};
use std::fmt;
use std::iter;
use std::sync::Arc;

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Utc};
use common::{Cooldown, Duration};
use diesel::backend::Backend;
use diesel::prelude::*;
use diesel::serialize::IsNull;
use diesel::serialize::ToSql;
use diesel::sqlite::Sqlite;
use diesel::{AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Authorization schema.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Schema {
    roles: HashMap<Role, RoleData>,
    scopes: HashMap<Scope, ScopeData>,
}

impl Schema {
    /// Load schema from the given set of bytes.
    pub fn load_static(schema: &[u8]) -> Result<Schema> {
        serde_yaml::from_slice(schema).context("failed to load auth.yaml")
    }
}

/// A role or a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleOrUser {
    Role(Role),
    User(String),
}

impl fmt::Display for RoleOrUser {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoleOrUser::User(user) => user.fmt(fmt),
            RoleOrUser::Role(role) => role.fmt(fmt),
        }
    }
}

impl std::str::FromStr for RoleOrUser {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix('@') {
            let role = Role::from_str(s)?;
            return Ok(RoleOrUser::Role(role));
        }

        Ok(RoleOrUser::User(db::user_id(s)))
    }
}

/// The kind of temporary grant.
#[derive(Debug, Clone, Copy)]
pub enum TemporaryKind {
    Allow,
    Deny,
}

/// A grant that has been temporarily given.
struct Temporary {
    pub(crate) scope: Scope,
    pub(crate) principal: RoleOrUser,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) kind: TemporaryKind,
}

impl Temporary {
    /// Test if the grant is expired.
    pub(crate) fn is_expired(&self, now: &DateTime<Utc>) -> bool {
        *now >= self.expires_at
    }
}

struct Inner {
    db: db::Database,
    /// Schema for every corresponding scope.
    schema: Schema,
    /// Assignments.
    grants: RwLock<HashSet<(Scope, Role)>>,
    /// Temporary grants.
    temporary: RwLock<Vec<Temporary>>,
}

/// A container for scopes and their grants.
#[derive(Clone)]
pub struct Auth {
    inner: Arc<Inner>,
}

impl Auth {
    /// Construct a new authorization handle.
    pub async fn new(db: db::Database, schema: Schema) -> Result<Self> {
        use db::schema::grants::dsl;

        let grants = db
            .asyncify(move |c| {
                let grants = dsl::grants
                    .select((dsl::scope, dsl::role))
                    .load::<(Scope, Role)>(c)?
                    .into_iter()
                    .collect::<HashSet<_>>();
                Ok::<_, Error>(grants)
            })
            .await?;

        let auth = Auth {
            inner: Arc::new(Inner {
                db,
                schema,
                grants: RwLock::new(grants),
                temporary: Default::default(),
            }),
        };

        // perform default initialization based on auth.yaml
        auth.insert_default_grants().await?;
        Ok(auth)
    }

    /// Return all temporary scopes belonging to the specified user.
    async fn temporary_scopes(&self, now: &DateTime<Utc>, principal: RoleOrUser) -> Vec<Scope> {
        let mut out = Vec::new();

        let grants = self.inner.temporary.read().await;

        for grant in grants.iter() {
            if grant.principal == principal && !grant.is_expired(now) {
                out.push(grant.scope);
            }
        }

        out
    }

    /// Return all temporary scopes belonging to the specified user.
    pub async fn scopes_for_user(&self, user: &str) -> Vec<Scope> {
        let now = Utc::now();
        self.temporary_scopes(&now, RoleOrUser::User(user.to_string()))
            .await
    }

    /// Return all temporary scopes belonging to the specified user.
    pub async fn scopes_for_role(&self, needle: Role) -> Vec<Scope> {
        let now = Utc::now();
        let mut out = self.temporary_scopes(&now, RoleOrUser::Role(needle)).await;

        let grants = self.inner.grants.read().await;

        for (scope, role) in grants.iter() {
            if needle == *role {
                out.push(*scope);
            }
        }

        out
    }

    /// Construct scope cooldowns.
    pub fn scope_cooldowns(&self) -> HashMap<Scope, Cooldown> {
        let mut cooldowns = HashMap::new();

        for (scope, schema) in self.inner.schema.scopes.iter() {
            if let Some(duration) = schema.cooldown {
                cooldowns.insert(*scope, Cooldown::from_duration(duration));
            }
        }

        cooldowns
    }

    /// Insert default grants for non-initialized grants.
    async fn insert_default_grants(&self) -> Result<()> {
        use db::schema::initialized_grants::dsl;

        let grants = self
            .inner
            .db
            .asyncify(move |c| {
                let grants = dsl::initialized_grants
                    .select((dsl::scope, dsl::version))
                    .load::<(Scope, String)>(c)?
                    .into_iter()
                    .collect::<HashMap<Scope, String>>();
                Ok::<_, Error>(grants)
            })
            .await?;

        let mut to_insert = Vec::new();

        for (key, data) in &self.inner.schema.scopes {
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
                self.insert(key, *allow).await?;
            }

            let version = data.version.clone();

            self.inner
                .db
                .asyncify(move |c| {
                    diesel::insert_into(dsl::initialized_grants)
                        .values((dsl::scope.eq(key), dsl::version.eq(version)))
                        .execute(c)?;
                    Ok::<_, Error>(())
                })
                .await?;
        }

        Ok(())
    }

    /// Insert a temporary grant.
    pub async fn insert_temporary(
        &self,
        scope: Scope,
        principal: RoleOrUser,
        expires_at: DateTime<Utc>,
        kind: TemporaryKind,
    ) {
        let mut grants = self.inner.temporary.write().await;

        if let Some(existing) = grants
            .iter_mut()
            .find(|g| g.scope == scope && g.principal == principal)
        {
            existing.expires_at = expires_at;
            existing.kind = kind;
        } else {
            grants.push(Temporary {
                scope,
                principal,
                expires_at,
                kind,
            });
        }
    }

    /// Insert an assignment.
    pub async fn insert(&self, scope: Scope, role: Role) -> Result<()> {
        use db::schema::grants::dsl;

        self.inner
            .db
            .asyncify(move |c| {
                diesel::insert_into(dsl::grants)
                    .values((dsl::scope.eq(scope), dsl::role.eq(role)))
                    .execute(c)?;
                Ok::<_, Error>(())
            })
            .await?;

        self.inner.grants.write().await.insert((scope, role));
        Ok(())
    }

    /// Delete an assignment.
    pub async fn delete(&self, scope: Scope, role: Role) -> Result<()> {
        use db::schema::grants::dsl;

        if self.inner.grants.write().await.remove(&(scope, role)) {
            self.inner
                .db
                .asyncify(move |c| {
                    let _ = diesel::delete(
                        dsl::grants.filter(dsl::scope.eq(scope).and(dsl::role.eq(role))),
                    )
                    .execute(c)?;
                    Ok::<_, Error>(())
                })
                .await?;
        }

        Ok(())
    }

    /// Test if there are any temporary grants matching the given user or role.
    async fn test_temporary(
        &self,
        now: &DateTime<Utc>,
        scope: &Scope,
        against: impl IntoIterator<Item = RoleOrUser>,
    ) -> (Option<TemporaryKind>, bool) {
        let temporary = self.inner.temporary.read().await;

        if temporary.is_empty() {
            return (None, false);
        }

        let mut granted = None;
        let mut expired = false;

        'outer: for against in against {
            for t in temporary.iter() {
                if t.principal != against || t.scope != *scope {
                    continue;
                }

                if t.is_expired(now) {
                    expired = true;
                    continue;
                }

                granted = Some(t.kind);
                break 'outer;
            }
        }

        (granted, expired)
    }

    /// Test if the given assignment exists.
    pub async fn test_any<S>(
        &self,
        scope: S,
        user: &str,
        roles: impl IntoIterator<Item = Role>,
    ) -> bool
    where
        S: AsRef<Scope>,
    {
        let scope = scope.as_ref();
        let roles = roles.into_iter().collect::<Vec<_>>();

        let now = Utc::now();

        let against = iter::once(RoleOrUser::User(user.to_string()))
            .chain(roles.iter().copied().map(RoleOrUser::Role));

        let (grant, expired) = self.test_temporary(&now, scope, against).await;

        // Delete temporary grants that has expired.
        if expired {
            self.inner
                .temporary
                .write()
                .await
                .retain(|g| !g.is_expired(&now));
        }

        let outcome = 'outcome: {
            if !matches!(grant, Some(TemporaryKind::Deny)) {
                let grants = self.inner.grants.read().await;

                if roles.iter().any(|r| grants.contains(&(*scope, *r))) {
                    break 'outcome true;
                }
            }

            matches!(grant, Some(TemporaryKind::Allow))
        };

        tracing::info!(
            ?grant,
            ?expired,
            ?scope,
            ?user,
            ?roles,
            ?outcome,
            "tested scopes"
        );

        outcome
    }

    /// Get a list of scopes and extra information associated with them.
    pub fn scopes(&self) -> Vec<ScopeInfo> {
        let mut out = Vec::new();

        for scope in Scope::list() {
            let data = match self.inner.schema.scopes.get(&scope) {
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
            let data = match self.inner.schema.roles.get(&role) {
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
    pub async fn list(&self) -> Vec<(Scope, Role)> {
        self.inner.grants.read().await.iter().cloned().collect()
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
            Serialize,
            Deserialize,
            FromSqlRow,
            AsExpression,
        )]
        #[diesel(sql_type = diesel::sql_types::Text)]
        pub enum Scope {
            $(#[serde(rename = $scope)] $variant,)*
            Unknown,
        }

        impl settings::Scope for Scope {
        }

        impl Default for Scope {
            fn default() -> Self {
                Self::Unknown
            }
        }

        impl Scope {
            /// Get a list of all scopes.
            pub(crate) fn list() -> Vec<Scope> {
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

        impl ToSql<diesel::sql_types::Text, Sqlite> for Scope {
            fn to_sql(&self, out: &mut diesel::serialize::Output<'_, '_, Sqlite>) -> diesel::serialize::Result {
                out.set_value(self.to_string());
                Ok(IsNull::No)
            }
        }

        impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for Scope
        where
            DB: Backend,
            String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
        {
            fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
                let s = String::from_sql(bytes)?;
                Ok(str::parse(&s)?)
            }
        }

        impl AsRef<Scope> for Scope {
            #[inline]
            fn as_ref(&self) -> &Scope {
                self
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
        Serialize,
        Deserialize,
        FromSqlRow,
        AsExpression,
    )]
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub enum Role {
        $(#[serde(rename = $role)] $variant,)*
        Unknown,
    }

    impl Role {
        /// Get a list of all roles.
        pub(crate) fn list() -> Vec<Role> {
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

    impl ToSql<diesel::sql_types::Text, Sqlite> for Role {
        fn to_sql(&self, out: &mut diesel::serialize::Output<'_, '_, Sqlite>) -> diesel::serialize::Result {
            out.set_value(self.to_string());
            Ok(IsNull::No)
        }
    }

    impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for Role
    where
        DB: Backend,
        String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
            let s = String::from_sql(bytes)?;
            Ok(str::parse(&s)?)
        }
    }
    }
}

/// The risk of a given scope.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Default)]
pub(crate) enum Risk {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "default", other)]
    #[default]
    Default,
}

scopes! {
    (BypassCooldowns, "bypass-cooldowns"),
    (PlayerDetachDetach, "player/attach-detach"),
    (Admin, "admin"),
    (Song, "song"),
    (SongYouTube, "song/youtube"),
    (SongSpotify, "song/spotify"),
    (SongBypassConstraints, "song/bypass-constraints"),
    (SongTheme, "song/theme"),
    (SongEditQueue, "song/edit-queue"),
    (SongListLimit, "song/list-limit"),
    (SongVolume, "song/volume"),
    (SongPlaybackControl, "song/playback-control"),
    (SwearJar, "swearjar"),
    (Uptime, "uptime"),
    (Game, "game"),
    (GameEdit, "game/edit"),
    (Title, "title"),
    (TitleEdit, "title/edit"),
    (AfterStream, "afterstream"),
    (Clip, "clip"),
    (EightBall, "8ball"),
    (Command, "command"),
    (CommandEdit, "command/edit"),
    (ThemeEdit, "theme/edit"),
    (PromoEdit, "promo/edit"),
    (AliasEdit, "alias/edit"),
    (Countdown, "countdown"),
    (GtavBypassCooldown, "gtav/bypass-cooldown"),
    (GtavRaw, "gtav/raw"),
    (Speedrun, "speedrun"),
    (CurrencyShow, "currency/show"),
    (CurrencyBoost, "currency/boost"),
    (CurrencyWindfall, "currency/windfall"),
    (WaterUndo, "water/undo"),
    (AuthPermit, "auth/permit"),
    (ChatBypassUrlWhitelist, "chat/bypass-url-whitelist"),
    (Time, "time"),
    (Poll, "poll"),
    (Weather, "weather"),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeInfo {
    scope: Scope,
    #[serde(flatten)]
    data: ScopeData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ScopeData {
    /// Documentation for this scope.
    pub(crate) doc: String,
    /// How risky is this scope to grant.
    /// High risk grants should be prompted with a warning before granted.
    #[serde(default)]
    pub(crate) risk: Risk,
    /// Version of the schema.
    /// A change in version will cause the default grants to be applied.
    pub(crate) version: String,
    /// Default grants for the scope.
    pub(crate) allow: Vec<Role>,
    /// Cooldown in effect for the given scope.
    pub(crate) cooldown: Option<Duration>,
}

roles! {
    (Streamer, "@streamer"),
    (Moderator, "@moderator"),
    (Subscriber, "@subscriber"),
    (Vip, "@vip"),
    (Everyone, "@everyone"),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleInfo {
    role: Role,
    #[serde(flatten)]
    data: RoleData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct RoleData {
    /// Documentation for this role.
    pub(crate) doc: String,
}
