//! Utilities for dealing with dynamic configuration and settings.

use crate::{db, utils};
use diesel::prelude::*;
use futures::{sync::mpsc, Async, Future as _, Poll, Stream as _};
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::{fmt, sync::Arc};
use tokio_core::reactor::Core;

const SEPARATOR: &'static str = "/";

type EventSender = mpsc::UnboundedSender<Event<serde_json::Value>>;
type Subscriptions = Arc<RwLock<HashMap<String, EventSender>>>;

/// Update events for a given key.
#[derive(Clone)]
pub enum Event<T> {
    /// Indicate that the given key was cleared.
    Clear,
    /// Indicate that the given key was updated.
    Set(T),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Setting {
    schema: SchemaType,
    key: String,
    value: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SchemaType {
    /// Documentation for this type.
    pub doc: String,
    /// The type.
    #[serde(rename = "type")]
    pub ty: Type,
}

const SCHEMA: &'static [u8] = include_bytes!("settings.yaml");

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Schema {
    types: HashMap<String, SchemaType>,
}

impl Schema {
    /// Load schema from the given set of bytes.
    pub fn load_static() -> Result<Schema, failure::Error> {
        Ok(serde_yaml::from_slice(SCHEMA)?)
    }

    /// Lookup the given type by key.
    pub fn lookup(&self, key: &str) -> Option<SchemaType> {
        self.types.get(key).cloned()
    }

    /// Test if schema contains the given key.
    pub fn contains(&self, key: &str) -> bool {
        self.types.contains_key(key)
    }
}

/// A container for settings from which we can subscribe for updates.
#[derive(Clone)]
pub struct Settings {
    db: db::Database,
    /// Maps setting prefixes to subscriptions.
    subscriptions: Subscriptions,
    /// Schema for every corresponding type.
    pub schema: Arc<Schema>,
}

impl Settings {
    pub fn new(db: db::Database, schema: Schema) -> Self {
        Self {
            db,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            schema: Arc::new(schema),
        }
    }

    /// Get a setting by prefix.
    pub fn get_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<(String, serde_json::Value)>, failure::Error> {
        use self::db::schema::settings::dsl;
        let c = self.db.pool.lock();

        let results = dsl::settings
            .select((dsl::key, dsl::value))
            .load::<(String, String)>(&*c)?;

        let mut out = Vec::new();

        for (key, value) in results {
            if !key.starts_with(prefix) {
                continue;
            }

            let value = serde_json::from_str(value.as_str())?;
            out.push((key, value));
        }

        return Ok(out);
    }

    /// Get the value of the given key from the database.
    pub fn get<T>(&self, key: &str) -> Result<Option<T>, failure::Error>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        use self::db::schema::settings::dsl;
        let c = self.db.pool.lock();

        let result = dsl::settings
            .select(dsl::value)
            .filter(dsl::key.eq(key))
            .first::<String>(&*c)
            .optional()?;

        let value = match result {
            Some(value) => match serde_json::from_str(&value) {
                Ok(value) => Some(value),
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
    pub fn set<T>(&self, key: &str, value: T) -> Result<(), failure::Error>
    where
        T: serde::Serialize,
    {
        let value = serde_json::to_value(value)?;
        self.set_json(key, value)
    }

    /// Insert the given setting as raw JSON.
    pub fn set_json(&self, key: &str, value: serde_json::Value) -> Result<(), failure::Error> {
        use self::db::schema::settings::dsl;

        {
            let subscriptions = self.subscriptions.read();

            if let Some(sub) = subscriptions.get(key) {
                if log::log_enabled!(log::Level::Trace) {
                    let level = serde_json::to_string(&value)?;
                    log::trace!("send: {} = {}", key, level);
                }

                if let Err(_) = sub.unbounded_send(Event::Set(value.clone())) {
                    log::error!("failed to send message to subscription on: {}", key);
                }
            }
        }

        let c = self.db.pool.lock();

        let filter = dsl::settings.filter(dsl::key.eq(&key));

        let b = filter
            .clone()
            .select((dsl::key, dsl::value))
            .first::<(String, String)>(&*c)
            .optional()?;

        let value = serde_json::to_string(&value)?;

        match b {
            None => {
                diesel::insert_into(dsl::settings)
                    .values((dsl::key.eq(key), dsl::value.eq(value)))
                    .execute(&*c)?;
            }
            Some(_) => {
                diesel::update(filter)
                    .set((dsl::key.eq(key), dsl::value.eq(&value)))
                    .execute(&*c)?;
            }
        }

        Ok(())
    }

    /// Insert the given setting.
    pub fn list(&self) -> Result<Vec<Setting>, failure::Error> {
        use self::db::schema::settings::dsl;
        let c = self.db.pool.lock();

        let mut settings = Vec::new();

        for (key, value) in dsl::settings
            .select((dsl::key, dsl::value))
            .order(dsl::key)
            .load::<(String, String)>(&*c)?
        {
            let value = serde_json::from_str(&value)?;

            let schema = match self.schema.lookup(&key) {
                Some(schema) => schema,
                // NB: skip over unknown keys.
                None => continue,
            };

            settings.push(Setting {
                schema,
                key: key.to_string(),
                value,
            });
        }

        Ok(settings)
    }

    /// Clear the given setting. Returning `true` if it was removed.
    pub fn clear(&self, key: &str) -> Result<bool, failure::Error> {
        use self::db::schema::settings::dsl;

        {
            let subscriptions = self.subscriptions.read();

            if let Some(sub) = subscriptions.get(key) {
                if let Err(_) = sub.unbounded_send(Event::Clear) {
                    log::error!("failed to send message to subscription on: {}", key);
                }
            }
        }

        let c = self.db.pool.lock();
        let count = diesel::delete(dsl::settings.filter(dsl::key.eq(key))).execute(&*c)?;
        Ok(count == 1)
    }

    /// Create a scoped setting.
    pub fn scoped<S>(&self, scope: impl IntoIterator<Item = S>) -> ScopedSettings
    where
        S: AsRef<str>,
    {
        let scope = scope
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect::<Vec<_>>();

        ScopedSettings {
            settings: self.clone(),
            scope,
        }
    }

    /// Subscribe for events on the given key.
    pub fn stream<T>(&self, key: &str, default: T) -> Stream<T>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        let (tx, rx) = mpsc::unbounded();

        if !self.schema.contains(key) {
            panic!("no schema registered for key `{}`", key);
        }

        {
            let mut subscriptions = self.subscriptions.write();

            if subscriptions.insert(key.to_string(), tx).is_some() {
                panic!("already a subscription for key: {}", key);
            }
        }

        Stream {
            default,
            subscriptions: self.subscriptions.clone(),
            key: key.to_string(),
            rx,
        }
    }

    /// Initialize the value from the database.
    pub fn init_and_stream<T>(
        &self,
        key: &str,
        default: T,
    ) -> Result<(Stream<T>, T), failure::Error>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        let value = match self.get::<T>(key)? {
            Some(value) => value,
            None => {
                self.set(key, &default)?;
                default.clone()
            }
        };

        Ok((self.stream(key, default), value))
    }

    /// Get a synchronized variable for the given configuration key.
    pub fn sync_var<T>(
        &self,
        core: &mut Core,
        key: &str,
        default: T,
    ) -> Result<Arc<RwLock<T>>, failure::Error>
    where
        T: 'static
            + fmt::Debug
            + Send
            + Sync
            + Clone
            + serde::Serialize
            + serde::de::DeserializeOwned,
    {
        let (stream, value) = self.init_and_stream(key, default)?;

        let value = Arc::new(RwLock::new(value));

        let future = stream.for_each({
            let key = key.to_string();
            let value = value.clone();

            move |update| {
                log::trace!("Updating: {} = {:?}", key, value);
                *value.write() = update;
                Ok(())
            }
        });

        core.runtime().executor().spawn(future.or_else(|e| {
            log::error!("sync_var update future failed: {}", e);
            Ok(())
        }));

        Ok(value)
    }
}

#[derive(Clone)]
pub struct ScopedSettings {
    settings: Settings,
    scope: Vec<String>,
}

impl ScopedSettings {
    /// Get the value of the given key from the database.
    pub fn get<T>(&self, key: &str) -> Result<Option<T>, failure::Error>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        self.settings.get(&self.scope(key))
    }

    /// Insert the given setting.
    pub fn set<T>(&self, key: &str, value: &T) -> Result<(), failure::Error>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        self.settings.set(&self.scope(key), value)
    }

    /// Clear the given setting. Returning `true` if it was removed.
    pub fn clear(&self, key: &str) -> Result<bool, failure::Error> {
        self.settings.clear(&self.scope(key))
    }

    /// Subscribe for events on the given key.
    pub fn stream<T>(&self, key: &str, default: T) -> Stream<T>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        self.settings.stream(&self.scope(key), default)
    }

    fn scope(&self, key: &str) -> String {
        let mut scope = self.scope.clone();
        scope.push(key.to_string());
        scope.join(SEPARATOR)
    }

    /// Initialize the value from the database.
    pub fn init_and_stream<T>(
        &self,
        key: &str,
        default: T,
    ) -> Result<(Stream<T>, T), failure::Error>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        self.settings.init_and_stream(&self.scope(key), default)
    }

    /// Get a synchronized variable for the given configuration key.
    pub fn sync_var<T>(
        &self,
        core: &mut Core,
        key: &str,
        default: T,
    ) -> Result<Arc<RwLock<T>>, failure::Error>
    where
        T: 'static
            + fmt::Debug
            + Send
            + Sync
            + Clone
            + serde::Serialize
            + serde::de::DeserializeOwned,
    {
        self.settings.sync_var(core, &self.scope(key), default)
    }

    /// Scope the settings a bit more.
    pub fn scoped<S>(&self, add: impl IntoIterator<Item = S>) -> ScopedSettings
    where
        S: AsRef<str>,
    {
        let mut scope = self.scope.clone();
        scope.extend(add.into_iter().map(|s| s.as_ref().to_string()));

        ScopedSettings {
            settings: self.settings.clone(),
            scope,
        }
    }
}

/// Get updates for a specific setting.
pub struct Stream<T> {
    default: T,
    subscriptions: Subscriptions,
    key: String,
    rx: mpsc::UnboundedReceiver<Event<serde_json::Value>>,
}

impl<T> Drop for Stream<T> {
    fn drop(&mut self) {
        if self.subscriptions.write().remove(&self.key).is_some() {
            return;
        }

        log::warn!(
            "Subscription dropped, but failed to clean up Settings for key: {}",
            self.key
        );
    }
}

#[derive(Debug, err_derive::Error)]
pub enum StreamError {
    #[error(display = "update stream errored")]
    UpdateStreamErrored,
    #[error(display = "update stream ended")]
    UpdateStreamEnded,
}

impl<T> futures::Stream for Stream<T>
where
    T: Clone + serde::de::DeserializeOwned,
{
    type Item = T;
    type Error = StreamError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        log::trace!("polling stream: {}", self.key);

        loop {
            let update = match self.rx.poll() {
                Err(()) => return Err(StreamError::UpdateStreamErrored),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Ok(Async::Ready(None)) => return Err(StreamError::UpdateStreamEnded),
                Ok(Async::Ready(Some(update))) => update,
            };

            let value = match update {
                Event::Clear => self.default.clone(),
                Event::Set(value) => {
                    let value = match serde_json::from_value(value) {
                        Ok(value) => value,
                        Err(e) => {
                            log::warn!("bad value for key: {}: {}", self.key, e);
                            continue;
                        }
                    };

                    value
                }
            };

            return Ok(Async::Ready(Some(value)));
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "id")]
pub enum Type {
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "duration")]
    Duration,
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "set")]
    Set { value: Box<Type> },
}

impl Type {
    /// Construct a set with the specified inner value.
    pub fn set(value: Type) -> Type {
        Type::Set {
            value: Box::new(value),
        }
    }

    /// Parse the given string as the current type and convert into JSON.
    pub fn parse_as_json(&self, s: &str) -> Result<serde_json::Value, failure::Error> {
        use self::Type::*;
        use serde_json::Value;

        let value = match *self {
            Raw => serde_json::from_str(s)?,
            Duration => {
                let d = str::parse::<utils::Duration>(s)?;
                Value::String(d.to_string())
            }
            Bool => Value::Bool(str::parse::<bool>(s)?),
            Number => {
                let n = str::parse::<serde_json::Number>(s)?;
                Value::Number(n)
            }
            String => Value::String(s.to_string()),
            Set { ref value } => {
                let json = serde_json::from_str(s)?;

                match json {
                    Value::Array(values) => {
                        if !values.iter().all(|v| value.is_compatible_with_json(v)) {
                            failure::bail!("expected {}", self);
                        }

                        Value::Array(values)
                    }
                    _ => failure::bail!("expected array"),
                }
            }
        };

        Ok(value)
    }

    /// Test if JSON value is compatible with the current type.
    pub fn is_compatible_with_json(&self, other: &serde_json::Value) -> bool {
        use self::Type::*;
        use serde_json::Value;

        match (self, other) {
            (Raw, _) => true,
            (Duration, Value::Number(..)) => true,
            (Bool, Value::Bool(..)) => true,
            (Number, Value::Number(..)) => true,
            (String, Value::String(..)) => true,
            (Set { ref value }, Value::Array(ref values)) => {
                values.iter().all(|v| value.is_compatible_with_json(v))
            }
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Type::*;

        match *self {
            Raw => write!(fmt, "any"),
            Duration => write!(fmt, "duration"),
            Bool => write!(fmt, "bool"),
            Number => write!(fmt, "number"),
            String => write!(fmt, "string"),
            Set { ref value } => write!(fmt, "Array<{}>", value),
        }
    }
}
