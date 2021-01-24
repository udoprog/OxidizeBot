//! Utilities for dealing with dynamic configuration and settings.

use crate::db;
use crate::prelude::*;
use crate::utils;
use chrono_tz::Tz;
use diesel::prelude::*;
use serde::de;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::marker;
use std::ops;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinError;

/// Separator in configuration hierarchy.
const SEP: char = '/';

/// Indication that a value has been updated.
type Update = Event<serde_json::Value>;

/// Required traits for a scope.
pub trait Scope: 'static + Clone + Send + Sync + Default {}

/// A synchronized variable from settings.
#[derive(Debug)]
pub struct Var<T> {
    value: Arc<RwLock<T>>,
}

impl<T> Var<T> {
    /// Construct a new var.
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
        }
    }

    /// Load the given var.
    pub async fn load(&self) -> T
    where
        T: Clone,
    {
        self.value.read().await.clone()
    }

    /// Read the synchronized var.
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.value.read().await
    }

    /// Write to the synchronized var.
    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.value.write().await
    }
}

impl<T> Clone for Var<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// Failed to perform work due to injector shutting down.
    Shutdown,
    /// Unexpected end of driver stream.
    EndOfDriverStream,
    /// Driver already configured.
    DriverAlreadyConfigured,
    /// No target for schema key.
    NoTargetForSchema(String),
    /// Invalid time zones.
    InvalidTimeZone(String),
    /// Missing a required field.
    MissingRequiredField(String),
    /// Expected JSON object.
    Expected(&'static str),
    /// Expected a specific type.
    ExpectedType(Type),
    /// Incompatible field encountered.
    IncompatibleField(String),
    /// Expected one of the specified types.
    ExpectedOneOf(Vec<String>),
    /// Migration compatibility issues.
    MigrationIncompatible {
        from: String,
        to: String,
        ty: Type,
        json: String,
    },
    /// JSON error.
    Json(serde_json::Error),
    /// Diesel error.
    Diesel(diesel::result::Error),
    /// Convert from a anyhow::Error.
    Error(anyhow::Error),
    /// Failed to load schema.
    FailedToLoadSchema(serde_yaml::Error),
    /// Bad boolean value.
    BadBoolean(std::str::ParseBoolError),
    /// Background task failed.
    TaskError(JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shutdown => "injector is shutting down".fmt(fmt),
            Self::EndOfDriverStream => "end of driver stream".fmt(fmt),
            Self::DriverAlreadyConfigured => "driver already configured".fmt(fmt),
            Self::NoTargetForSchema(key) => write!(fmt, "no target for schema key: {}", key),
            Self::InvalidTimeZone(tz) => write!(fmt, "Invalid time zone: {}", tz,),
            Self::MissingRequiredField(field) => {
                write!(fmt, "Missing required field: {}", field)
            }
            Self::Expected(what) => write!(fmt, "Expected {}", what),
            Self::ExpectedType(ty) => write!(fmt, "Expected type: {}", ty),
            Self::IncompatibleField(field) => write!(fmt, "Incompatible field: {}", field),
            Self::ExpectedOneOf(alts) => write!(fmt, "Expected one of: {}", alts.join(", ")),
            Self::MigrationIncompatible { from, to, ty, json } => write!(
                fmt,
                "value for {} (json: {}) is not compatible with {} ({})",
                from, json, to, ty
            ),
            Self::Json(e) => write!(fmt, "JSON Error: {}", e),
            Self::Diesel(e) => write!(fmt, "Diesel Error: {}", e),
            Self::Error(e) => write!(fmt, "Error: {}", e),
            Self::FailedToLoadSchema(e) => write!(fmt, "Failed to load settings.yaml: {}", e),
            Self::BadBoolean(e) => write!(fmt, "Bad boolean value: {}", e),
            Self::TaskError(..) => write!(fmt, "Task failed"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::Json(ref e) => Some(e),
            Self::Diesel(ref e) => Some(e),
            Self::TaskError(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Self {
        Error::Diesel(e)
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Error(e)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Error::TaskError(e)
    }
}

/// Update events for a given key.
#[derive(Debug, Clone)]
pub enum Event<T> {
    /// Indicate that the given key was cleared.
    Clear,
    /// Indicate that the given key was updated.
    Set(T),
}

#[derive(Debug, Clone, Serialize)]
pub struct Setting<S>
where
    S: Scope,
{
    pub schema: SchemaType<S>,
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingRef<'settings, 'key, S, T>
where
    S: Scope,
{
    schema: &'settings SchemaType<S>,
    key: Key<'settings, 'key>,
    value: Option<T>,
}

impl<S> SettingRef<'_, '_, S, serde_json::Value>
where
    S: Scope,
{
    /// Convert into an owned value.
    pub fn to_owned(&self) -> Setting<S> {
        Setting {
            schema: self.schema.clone(),
            key: self.key.to_string(),
            value: match self.value.clone() {
                None => serde_json::Value::Null,
                Some(value) => value,
            },
        }
    }
}

impl<S, T> SettingRef<'_, '_, S, T>
where
    S: Scope,
{
    /// Access the underlying key this setting references.
    pub fn key(&self) -> &str {
        &*self.key
    }

    /// Access the underlying schema this setting references.
    pub fn schema(&self) -> &SchemaType<S> {
        self.schema
    }

    /// Access the raw value this setting references.
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaType<S>
where
    S: Scope,
{
    /// Documentation for this type.
    pub doc: String,
    /// Scope required to modify variable.
    #[serde(default)]
    pub scope: Option<S>,
    /// The type.
    #[serde(rename = "type")]
    pub ty: Type,
    /// If the value is a secret value or not.
    #[serde(default)]
    pub secret: bool,
    /// If the setting is a feature toggle.
    #[serde(default)]
    pub feature: bool,
    /// A human-readable title for the setting.
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Schema<S>
where
    S: Scope,
{
    #[serde(default)]
    migrations: Vec<Migration>,
    types: HashMap<String, SchemaType<S>>,
}

impl<S> Schema<S>
where
    S: Scope,
{
    /// Convert schema into prefix data.
    fn as_prefixes(&self) -> HashMap<Box<str>, Prefix> {
        let mut prefixes = HashMap::<Box<str>, Prefix>::new();

        for key in self.types.keys() {
            let mut buf = String::new();

            let mut p = key.split(SEP).peekable();

            while let Some(part) = p.next() {
                buf.push_str(&part);

                prefixes
                    .entry(buf.as_str().into())
                    .or_default()
                    .keys
                    .push(key.clone());

                if p.peek().is_some() {
                    buf.push(SEP);
                }
            }
        }

        prefixes
    }

    fn as_subscriptions(&self) -> HashMap<Box<str>, broadcast::Sender<Update>> {
        let mut m = HashMap::new();

        for key in self.types.keys() {
            let (sender, _) = broadcast::channel(1);
            m.insert(key.as_str().into(), sender);
        }

        m
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Migration {
    /// Treat the migration as a prefix migration.
    #[serde(default)]
    pub prefix: bool,
    /// Key to migrate from.
    pub from: String,
    /// Key to migrate to.
    pub to: String,
}

impl<S> Schema<S>
where
    S: Scope + de::DeserializeOwned,
{
    /// Load schema from the given set of bytes.
    pub fn load_bytes(bytes: &[u8]) -> Result<Schema<S>, Error> {
        serde_yaml::from_slice(bytes).map_err(Error::FailedToLoadSchema)
    }
}

impl<S> Schema<S>
where
    S: Scope,
{
    /// Lookup the given type by key.
    pub fn lookup(&self, key: &str) -> Option<SchemaType<S>> {
        self.types.get(key).cloned()
    }

    /// Test if schema contains the given key.
    pub fn contains(&self, key: &str) -> bool {
        self.types.contains_key(key)
    }
}

/// Information on a given prefix.
#[derive(Default)]
struct Prefix {
    /// All keys that belongs to the given prefix.
    keys: Vec<String>,
}

/// The future that drives a synchronized variable.
struct Driver {
    future: Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>,
}

impl Future for Driver {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}

pub struct Inner<S>
where
    S: Scope,
{
    db: db::Database,
    /// Maps setting prefixes to subscriptions.
    subscriptions: HashMap<Box<str>, broadcast::Sender<Update>>,
    /// Schema for every corresponding type.
    schema: Schema<S>,
    /// Information about all prefixes.
    prefixes: HashMap<Box<str>, Prefix>,
    /// Channel where new drivers are sent.
    drivers: mpsc::UnboundedSender<Driver>,
    /// Receiver for drivers. Used by the run function.
    drivers_rx: Mutex<Option<mpsc::UnboundedReceiver<Driver>>>,
}

/// A container for settings from which we can subscribe for updates.
#[derive(Clone)]
pub struct Settings<S>
where
    S: Scope,
{
    scope: Box<str>,
    inner: Arc<Inner<S>>,
}

impl<S> Settings<S>
where
    S: Scope,
{
    pub fn new(db: db::Database, schema: Schema<S>) -> Self {
        let prefixes = schema.as_prefixes();
        let subscriptions = schema.as_subscriptions();
        let (drivers, drivers_rx) = mpsc::unbounded_channel();

        Self {
            scope: Default::default(),
            inner: Arc::new(Inner {
                db,
                subscriptions,
                schema,
                prefixes,
                drivers,
                drivers_rx: Mutex::new(Some(drivers_rx)),
            }),
        }
    }

    /// Run all settings migrations.
    pub async fn run_migrations(&self) -> Result<(), Error> {
        for m in &self.inner.schema.migrations {
            if m.prefix {
                self.migrate_prefix(&m.from, &m.to).await?;
            } else {
                self.migrate_exact(&m.from, &m.to).await?;
            }
        }

        Ok(())
    }

    /// Migrate one prefix to another.
    async fn migrate_prefix(&self, from_key: &str, to_key: &str) -> Result<(), Error> {
        use self::db::schema::settings::dsl;

        let keys = {
            self.inner
                .db
                .asyncify(|c| {
                    Ok::<_, Error>(
                        dsl::settings
                            .select(dsl::key)
                            .order(dsl::key)
                            .load::<String>(c)?
                            .into_iter()
                            .collect::<Vec<_>>(),
                    )
                })
                .await?
        };

        for current_key in keys {
            if !current_key.starts_with(from_key) {
                continue;
            }

            let to_key = format!("{}{}", to_key, &current_key[from_key.len()..]);

            match self.migrate_exact(&current_key, &to_key).await {
                Ok(()) => (),
                Err(Error::NoTargetForSchema(..)) => {
                    log::warn!("Clearing setting without schema: {}", current_key);
                    self.inner_clear(&current_key).await?;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// Migrate the exact key.
    async fn migrate_exact(&self, from_key: &str, to_key: &str) -> Result<(), Error> {
        let from = match self.inner_get::<serde_json::Value>(from_key).await? {
            Some(from) => from,
            None => return Ok(()),
        };

        log::info!("Migrating setting: {} -> {}", from_key, to_key);

        let to = match self.setting::<serde_json::Value>(to_key).await? {
            Some(to) => to,
            None => return Err(Error::NoTargetForSchema(to_key.to_string())),
        };

        if to.value.is_none() {
            if !to.schema.ty.is_compatible_with_json(&from) {
                return Err(Error::MigrationIncompatible {
                    from: from_key.to_string(),
                    to: to_key.to_string(),
                    ty: to.schema.ty.clone(),
                    json: serde_json::to_string(&from)?,
                });
            }

            self.set_json(&to_key, from).await?;
        } else {
            log::warn!(
                "Ignoring value for {} since {} is already present",
                from_key,
                to_key
            );
        }

        self.inner_clear(&from_key).await?;
        Ok(())
    }

    /// Lookup the given schema.
    pub fn lookup(&self, key: &str) -> Option<&SchemaType<S>> {
        let key = self.key(key);
        self.inner.schema.types.get(&*key)
    }

    /// Get a setting by prefix.
    pub async fn list_by_prefix(&self, prefix: &str) -> Result<Vec<Setting<S>>, Error> {
        use self::db::schema::settings::dsl;

        let prefix = self.key(prefix);
        let inner = self.inner.clone();
        let prefix = prefix.to_string();

        self.inner
            .db
            .asyncify(move |c| {
                let prefix = match inner.prefixes.get(prefix.as_str()) {
                    Some(prefix) => prefix,
                    None => return Ok(Vec::default()),
                };

                let mut settings = Vec::new();

                let values = dsl::settings
                    .select((dsl::key, dsl::value))
                    .order(dsl::key)
                    .load::<(String, String)>(c)?
                    .into_iter()
                    .collect::<HashMap<_, _>>();

                for key in &prefix.keys {
                    let schema = match inner.schema.types.get(key) {
                        Some(schema) => schema,
                        None => continue,
                    };

                    let value = match values.get(key) {
                        Some(value) => serde_json::from_str(value)?,
                        None if schema.ty.optional => serde_json::Value::Null,
                        None => continue,
                    };

                    settings.push(Setting {
                        schema: schema.clone(),
                        key: key.to_string(),
                        value,
                    });
                }

                Ok(settings)
            })
            .await
    }

    /// Get the given setting.
    ///
    /// This includes the schema of the setting as well.
    pub async fn setting<'settings, 'key, T>(
        &'settings self,
        key: &'key str,
    ) -> Result<Option<SettingRef<'settings, 'key, S, T>>, Error>
    where
        T: Serialize + de::DeserializeOwned,
    {
        let key = self.key(key);

        let schema = match self.inner.schema.types.get(&*key) {
            Some(schema) => schema,
            None => return Ok(None),
        };

        let value = self.inner_get(&*key).await?;
        Ok(Some(SettingRef { schema, key, value }))
    }

    /// Test if the given key exists in the database.
    pub async fn has(&self, key: &str) -> Result<bool, Error> {
        let key = self.key(key);
        Ok(self.inner_get::<serde_json::Value>(&key).await?.is_some())
    }

    /// Get the value of the given key from the database.
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T: Serialize + de::DeserializeOwned,
    {
        let key = self.key(key);
        self.inner_get(&key).await
    }

    /// Insert the given setting without sending an update notification to other components.
    pub async fn set_silent<T>(&self, key: &str, value: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let key = self.key(key);
        self.inner_set(key.as_ref(), value, false).await
    }

    /// Insert the given setting.
    pub async fn set<T>(&self, key: &str, value: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let key = self.key(key);
        self.inner_set(key.as_ref(), value, true).await
    }

    /// Insert the given setting as raw JSON.
    pub async fn set_json(&self, key: &str, value: serde_json::Value) -> Result<(), Error> {
        let key = self.key(key);
        self.inner_set_json(key.as_ref(), value, true).await
    }

    /// Inner implementation of set_json which doesn't do key translation.
    async fn inner_set_json(
        &self,
        key: &str,
        value: serde_json::Value,
        notify: bool,
    ) -> Result<(), Error> {
        use self::db::schema::settings::dsl;

        if log::log_enabled!(log::Level::Trace) {
            log::trace!("{}: Setting to {:?} (notify: {})", key, value, notify);
        }

        let key = key.to_string();

        let (key, value) = self
            .inner
            .db
            .asyncify(move |c| {
                let filter = dsl::settings.filter(dsl::key.eq(&key));

                let b = filter
                    .clone()
                    .select((dsl::key, dsl::value))
                    .first::<(String, String)>(c)
                    .optional()?;

                let json = serde_json::to_string(&value)?;

                match b {
                    None => {
                        diesel::insert_into(dsl::settings)
                            .values((dsl::key.eq(&key), dsl::value.eq(json)))
                            .execute(c)?;
                    }
                    Some(_) => {
                        diesel::update(filter)
                            .set((dsl::key.eq(&key), dsl::value.eq(json)))
                            .execute(c)?;
                    }
                }

                Ok::<_, Error>((key, value))
            })
            .await?;

        if notify {
            self.try_send(&key, Event::Set(value)).await;
        }

        Ok(())
    }

    /// Insert the given setting.
    pub async fn list(&self) -> Result<Vec<Setting<S>>, Error> {
        use self::db::schema::settings::dsl;

        let inner = self.inner.clone();

        self.inner
            .db
            .asyncify(move |c| {
                let mut settings = Vec::new();

                let values = dsl::settings
                    .select((dsl::key, dsl::value))
                    .order(dsl::key)
                    .load::<(String, String)>(c)?
                    .into_iter()
                    .collect::<HashMap<_, _>>();

                for (key, schema) in &inner.schema.types {
                    let value = match values.get(key) {
                        Some(value) => serde_json::from_str(value)?,
                        None if schema.ty.optional => serde_json::Value::Null,
                        None => continue,
                    };

                    settings.push(Setting {
                        schema: schema.clone(),
                        key: key.to_string(),
                        value,
                    });
                }

                Ok(settings)
            })
            .await
    }

    /// Clear the given setting. Returning `true` if it was removed.
    pub async fn clear(&self, key: &str) -> Result<bool, Error> {
        let key = self.key(key);
        self.inner_clear(&key).await
    }

    /// Perform an inner clear of the given key.
    async fn inner_clear(&self, key: &str) -> Result<bool, Error> {
        use self::db::schema::settings::dsl;

        let key = key.to_string();

        self.try_send(&key, Event::Clear).await;

        self.inner
            .db
            .asyncify(move |c| {
                let count = diesel::delete(dsl::settings.filter(dsl::key.eq(key))).execute(c)?;
                Ok(count == 1)
            })
            .await
    }

    /// Create a scoped setting.
    pub fn scoped(&self, s: &str) -> Settings<S> {
        let mut it = s.split('/').filter(|s| !s.is_empty());

        let scope = if self.scope.is_empty() {
            let mut scope = String::with_capacity(s.len());
            let last = it.next_back();

            while let Some(s) = it.next() {
                scope.push_str(s);
                scope.push(SEP);
            }

            if let Some(s) = last {
                scope.push_str(s);
            }

            scope
        } else {
            let mut scope = String::from(self.scope.as_ref());

            for s in it {
                scope.push(SEP);
                scope.push_str(s);
            }

            scope
        };

        Settings {
            scope: scope.into(),
            inner: self.inner.clone(),
        }
    }

    /// Initialize the value from the database.
    pub fn stream<'settings, 'key, T>(
        &'settings self,
        key: &'key str,
    ) -> StreamBuilder<'settings, 'key, S, T> {
        let key = self.key(key);

        StreamBuilder {
            settings: self,
            default_value: None,
            key,
        }
    }

    /// Get a synchronized variable for the given configuration key.
    pub async fn var<T>(&self, key: &str, default: T) -> Result<Var<T>, Error>
    where
        T: 'static + fmt::Debug + Send + Sync + Clone + Serialize + de::DeserializeOwned,
    {
        let (mut stream, value) = self.stream(key).or_with(default).await?;

        let var = Var::new(value);
        let future_var = var.clone();

        let future = Box::pin(async move {
            loop {
                let update = stream.recv().await;
                *future_var.write().await = update;
            }
        });

        let result = self.inner.drivers.send(Driver { future });

        if result.is_err() {
            return Err(Error::Shutdown);
        }

        Ok(var)
    }

    /// Get an optional synchronized variable for the given configuration key.
    pub async fn optional<T>(&self, key: &str) -> Result<Var<Option<T>>, Error>
    where
        T: 'static + fmt::Debug + Send + Sync + Clone + Serialize + de::DeserializeOwned,
    {
        let (mut stream, value) = self.stream(key).optional().await?;
        let value = Var::new(value);
        let future_value = value.clone();

        let future = Box::pin(async move {
            loop {
                let update = stream.recv().await;
                *future_value.write().await = update;
            }
        });

        let result = self.inner.drivers.send(Driver { future });

        if result.is_err() {
            return Err(Error::Shutdown);
        }

        Ok(value)
    }

    /// Run the injector as a future, making sure all asynchronous processes
    /// associated with it are driven to completion.
    ///
    /// This has to be called for the injector to perform important tasks.
    pub async fn drive(self) -> Result<(), Error> {
        let mut rx = self
            .inner
            .drivers_rx
            .lock()
            .await
            .take()
            .ok_or(Error::DriverAlreadyConfigured)?;

        let mut drivers = ::futures_util::stream::FuturesUnordered::new();

        loop {
            while drivers.is_empty() {
                drivers.push(rx.recv().await.ok_or(Error::EndOfDriverStream)?);
            }

            while !drivers.is_empty() {
                tokio::select! {
                    driver = rx.recv() => {
                        drivers.push(driver.ok_or(Error::EndOfDriverStream)?);
                    }
                    Some(()) = drivers.next() => (),
                }
            }
        }
    }

    /// Get the value of the given key from the database.
    async fn inner_get<T>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T: Serialize + de::DeserializeOwned,
    {
        use self::db::schema::settings::dsl;

        let inner_key = key.to_string();

        let result = self
            .inner
            .db
            .asyncify(move |c| {
                Ok::<_, Error>(
                    dsl::settings
                        .select(dsl::value)
                        .filter(dsl::key.eq(&inner_key))
                        .first::<String>(c)
                        .optional()?,
                )
            })
            .await?;

        let value = match result {
            Some(value) => match serde_json::from_str::<Option<T>>(&value) {
                Ok(value) => value,
                Err(e) => {
                    log::warn!("bad value for key: {}: {}", key, e);
                    None
                }
            },
            None => None,
        };

        Ok(value)
    }

    /// Insert the given setting.
    async fn inner_set<T>(&self, key: &str, value: T, notify: bool) -> Result<(), Error>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(value)?;
        self.inner_set_json(key, value, notify).await
    }

    /// Subscribe for events on the given key.
    async fn make_stream<T>(&self, key: &str, default: T) -> Stream<T>
    where
        T: Clone + Serialize + de::DeserializeOwned,
    {
        Stream {
            default,
            option_stream: self.make_option_stream(key).await,
        }
    }

    /// Subscribe for any events on the given key.
    async fn make_option_stream<T>(&self, key: &str) -> OptionStream<T>
    where
        T: Serialize + de::DeserializeOwned,
    {
        let rx = if let Some(sender) = self.inner.subscriptions.get(key) {
            sender.subscribe()
        } else {
            panic!("no schema registered for key `{}`", key);
        };

        OptionStream {
            key: key.into(),
            rx,
            marker: marker::PhantomData,
        }
    }

    /// Try to send the specified event.
    ///
    /// Cleans up the existing subscription if the other side is closed.
    async fn try_send(&self, key: &str, event: Update) {
        if let Some(b) = self.inner.subscriptions.get(key) {
            // NB: intentionally ignore errors. There's nothing to be done in
            // case there are any.
            let _ = b.send(event);
        }
    }

    /// Construct a new key.
    fn key<'settings, 'key>(&'settings self, key: &'key str) -> Key<'settings, 'key> {
        let key = key.trim_matches(SEP);

        if key.is_empty() {
            return Key::Settings(&self.scope);
        }

        if self.scope.is_empty() {
            Key::Key(key)
        } else {
            let mut scope = String::from(self.scope.as_ref());
            scope.push(SEP);
            scope.push_str(key);
            Key::Owned(scope.into())
        }
    }
}

/// Internal key holder, reduces the number of copies necessary when there's no
/// key specified or we can rely solely on scope.
#[derive(Clone)]
enum Key<'settings, 'key> {
    Settings(&'settings str),
    Key(&'key str),
    Owned(Box<str>),
}

impl fmt::Debug for Key<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl Serialize for Key<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ops::Deref::deref(self).serialize(serializer)
    }
}

impl ops::Deref for Key<'_, '_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Key::Settings(scope) => *scope,
            Key::Key(key) => *key,
            Key::Owned(key) => key.as_ref(),
        }
    }
}

#[must_use = "Must consume to drive decide how to handle stream"]
pub struct StreamBuilder<'settings, 'key, S, T>
where
    S: Scope,
{
    settings: &'settings Settings<S>,
    default_value: Option<T>,
    key: Key<'settings, 'key>,
}

impl<'settings, 'key, S, T> StreamBuilder<'settings, 'key, S, T>
where
    S: Scope,
    T: Serialize + de::DeserializeOwned,
{
    /// Make the setting required, falling back to using and storing the default value if necessary.
    pub async fn or_default(self) -> Result<(Stream<T>, T), Error>
    where
        T: Clone + Default,
    {
        self.or_with(T::default()).await
    }

    /// Make the setting required, falling back to using and storing the specified value if necessary.
    pub async fn or_with(self, value: T) -> Result<(Stream<T>, T), Error>
    where
        T: Clone,
    {
        self.or_with_else(move || value).await
    }

    /// Make the setting required, falling back to using and storing the specified value if necessary.
    pub async fn or_with_else<F>(self, value: F) -> Result<(Stream<T>, T), Error>
    where
        T: Clone,
        F: FnOnce() -> T,
    {
        let value = match self.settings.inner_get::<T>(&self.key).await? {
            Some(value) => value,
            None => {
                let value = value();
                self.settings.inner_set(&self.key, &value, true).await?;
                value
            }
        };

        let stream = self.settings.make_stream(&self.key, value.clone()).await;
        Ok((stream, value))
    }

    /// Make the setting optional.
    pub async fn optional(self) -> Result<(OptionStream<T>, Option<T>), Error> {
        let value = self.settings.inner_get::<T>(&self.key).await?;

        let value = match value {
            Some(value) => Some(value),
            None => match self.default_value {
                Some(value) => {
                    self.settings.inner_set(&self.key, &value, true).await?;
                    Some(value)
                }
                None => None,
            },
        };

        let stream = self.settings.make_option_stream(&self.key).await;
        Ok((stream, value))
    }

    /// Add a potential fallback value when the type is optional.
    pub fn or(self, other: Option<T>) -> Self {
        self.or_else(move || other)
    }

    /// Add a potential fallback value when the type is optional.
    pub fn or_else<F>(mut self, other: F) -> Self
    where
        F: FnOnce() -> Option<T>,
    {
        if self.default_value.is_some() {
            return self;
        }

        self.default_value = other();
        self
    }
}

/// Get updates for a specific setting.
pub struct Stream<T> {
    default: T,
    option_stream: OptionStream<T>,
}

impl<T> Stream<T>
where
    T: Clone + de::DeserializeOwned,
{
    /// Recv the next update to the setting associated with the stream.
    pub async fn recv(&mut self) -> T {
        if let Some(value) = self.option_stream.recv().await {
            value
        } else {
            self.default.clone()
        }
    }
}

/// Get updates for a specific setting.
pub struct OptionStream<T> {
    key: Box<str>,
    rx: broadcast::Receiver<Update>,
    marker: marker::PhantomData<T>,
}

impl<T> OptionStream<T>
where
    T: de::DeserializeOwned,
{
    /// Recv the next update to the setting associated with the stream.
    pub async fn recv(&mut self) -> Option<T> {
        let item = match self.rx.recv().await {
            Ok(item) => item,
            Err(e) => {
                log_warn!(e, "{}: stream reader errored", self.key);
                return None;
            }
        };

        match item {
            Event::Clear => None,
            Event::Set(value) => match serde_json::from_value(value) {
                Ok(value) => Some(value),
                Err(e) => {
                    log::warn!("bad value for key: {}: {}", self.key, e);
                    None
                }
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Format {
    #[serde(rename = "regex")]
    Regex { pattern: String },
    #[serde(rename = "time-zone")]
    TimeZone,
    #[serde(rename = "none")]
    None,
}

impl Default for Format {
    fn default() -> Self {
        Format::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Type {
    #[serde(default)]
    pub optional: bool,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub title: String,
    pub field: String,
    #[serde(rename = "type")]
    pub ty: Box<Type>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SelectVariant {
    #[serde(rename = "typeahead")]
    Typeahead,
    #[serde(rename = "default")]
    Default,
}

impl Default for SelectVariant {
    fn default() -> Self {
        SelectVariant::Default
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    title: String,
    value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "id")]
pub enum Kind {
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "duration")]
    Duration,
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "percentage")]
    Percentage,
    #[serde(rename = "string")]
    String {
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default)]
        format: Format,
    },
    /// String type that supports multi-line editing.
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "set")]
    Set { value: Box<Type> },
    #[serde(rename = "select")]
    Select {
        value: Box<Type>,
        options: Vec<SelectOption>,
        #[serde(default)]
        variant: SelectVariant,
    },
    #[serde(rename = "time-zone")]
    TimeZone,
    #[serde(rename = "object")]
    Object { fields: Vec<Field> },
}

impl Type {
    /// Construct a set with the specified inner value.
    pub fn set(ty: Type) -> Type {
        Type {
            optional: false,
            kind: Kind::Set {
                value: Box::new(ty),
            },
        }
    }

    /// Parse the given string as the current type and convert into JSON.
    pub fn parse_as_json(&self, s: &str) -> Result<serde_json::Value, Error> {
        use self::Kind::*;
        use serde_json::Value;

        if self.optional && s == "null" {
            return Ok(Value::Null);
        }

        let value = match self.kind {
            Raw => serde_json::from_str(s)?,
            Duration => {
                let d = str::parse::<utils::Duration>(s)?;
                Value::String(d.to_string())
            }
            Bool => Value::Bool(str::parse::<bool>(s).map_err(Error::BadBoolean)?),
            Number => {
                let n = str::parse::<serde_json::Number>(s)?;
                Value::Number(n)
            }
            Percentage => {
                let n = str::parse::<serde_json::Number>(s)?;
                Value::Number(n)
            }
            String { .. } | Text => Value::String(s.to_string()),
            Set { ref value } => {
                let json = serde_json::from_str(s)?;

                match json {
                    Value::Array(values) => {
                        if !values.iter().all(|v| value.is_compatible_with_json(v)) {
                            return Err(Error::ExpectedType(self.clone()));
                        }

                        Value::Array(values)
                    }
                    _ => return Err(Error::Expected("array")),
                }
            }
            Select {
                ref value,
                ref options,
                ..
            } => {
                let v = value.parse_as_json(s)?;

                if !options.iter().any(|o| o.value == v) {
                    let mut out = Vec::new();

                    for o in options {
                        out.push(serde_json::to_string(&o.value)?);
                    }

                    return Err(Error::ExpectedOneOf(out));
                }

                v
            }
            TimeZone => {
                let tz = str::parse::<Tz>(s).map_err(Error::InvalidTimeZone)?;
                Value::String(format!("{:?}", tz))
            }
            Object { ref fields, .. } => {
                let json = serde_json::from_str(s)?;

                let object = match json {
                    Value::Object(object) => object,
                    _ => return Err(Error::Expected("object")),
                };

                for field in fields {
                    let f = match object.get(&field.field) {
                        Some(f) => f,
                        None if field.ty.optional => continue,
                        None => return Err(Error::MissingRequiredField(field.field.clone())),
                    };

                    if !field.ty.is_compatible_with_json(f) {
                        return Err(Error::IncompatibleField(field.field.clone()));
                    }
                }

                Value::Object(object)
            }
        };

        Ok(value)
    }

    /// Test if JSON value is compatible with the current type.
    pub fn is_compatible_with_json(&self, other: &serde_json::Value) -> bool {
        use self::Kind::*;
        use serde_json::Value;

        if self.optional && *other == Value::Null {
            return true;
        }

        match (&self.kind, other) {
            (Raw, _) => true,
            (Duration, Value::String(ref s)) => str::parse::<utils::Duration>(s).is_ok(),
            (Bool, Value::Bool(..)) => true,
            (Number, Value::Number(..)) => true,
            (Percentage, Value::Number(..)) => true,
            (String { .. }, Value::String(..)) => true,
            (Text, Value::String(..)) => true,
            (Set { ref value }, Value::Array(ref values)) => {
                values.iter().all(|v| value.is_compatible_with_json(v))
            }
            (
                Select {
                    ref value,
                    ref options,
                    ..
                },
                json,
            ) => {
                if !value.is_compatible_with_json(json) {
                    return false;
                }

                options.iter().any(|opt| opt.value == *json)
            }
            (Object { ref fields, .. }, Value::Object(ref object)) => {
                // NB: check that all fields match expected schema.
                fields.iter().all(|f| match object.get(&f.field) {
                    Some(field) => f.ty.is_compatible_with_json(field),
                    None => f.ty.optional,
                })
            }
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Kind::*;

        match &self.kind {
            Raw => write!(f, "any")?,
            Duration => write!(f, "duration")?,
            Bool => write!(f, "bool")?,
            Number => write!(f, "number")?,
            Percentage => write!(f, "percentage")?,
            String { .. } => write!(f, "string")?,
            Text => write!(f, "text")?,
            Set { value } => write!(f, "Array<{}>", value)?,
            Select { value, options, .. } => {
                let options = options.iter().map(|o| &o.value).collect::<Vec<_>>();
                let options = serde_json::to_string(&options).map_err(|_| fmt::Error)?;
                write!(f, "Select<{}, one_of={}>", value, options)?;
            }
            TimeZone => write!(f, "TimeZone")?,
            Object { .. } => write!(f, "Object")?,
        };

        if self.optional {
            write!(f, "?")?;
        }

        Ok(())
    }
}
