//! Utilities for dealing with dynamic configuration and settings.

use crate::{auth::Scope, db, oauth2, prelude::*, utils};
use chrono_tz::Tz;
use diesel::prelude::*;
use failure::{bail, format_err, Error, ResultExt};
use futures::ready;
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::{borrow::Cow, fmt, marker, pin::Pin, sync::Arc};

const SEPARATOR: char = '/';

type EventSender = mpsc::UnboundedSender<Event<serde_json::Value>>;
type Subscriptions = Arc<RwLock<HashMap<String, Vec<EventSender>>>>;

/// Update events for a given key.
#[derive(Debug, Clone)]
pub enum Event<T> {
    /// Indicate that the given key was cleared.
    Clear,
    /// Indicate that the given key was updated.
    Set(T),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Setting {
    pub schema: SchemaType,
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SettingRef<'a, T> {
    pub schema: &'a SchemaType,
    pub key: Cow<'a, str>,
    pub value: Option<T>,
}

impl SettingRef<'_, serde_json::Value> {
    /// Convert into an owned value.
    pub fn to_setting(&self) -> Setting {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SchemaType {
    /// Documentation for this type.
    pub doc: String,
    /// Scope required to modify variable.
    #[serde(default)]
    pub scope: Option<Scope>,
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

const SCHEMA: &'static [u8] = include_bytes!("settings.yaml");

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Schema {
    #[serde(default)]
    migrations: Vec<Migration>,
    types: HashMap<String, SchemaType>,
}

impl Schema {
    /// Convert schema into prefix data.
    fn as_prefixes(&self) -> HashMap<String, Prefix> {
        let mut prefixes = HashMap::<String, Prefix>::new();

        for key in self.types.keys() {
            let mut buf = String::new();

            let mut p = key.split(SEPARATOR).peekable();

            while let Some(part) = p.next() {
                buf.push_str(&part);

                prefixes
                    .entry(buf.clone())
                    .or_default()
                    .keys
                    .push(key.clone());

                if p.peek().is_some() {
                    buf.push(SEPARATOR);
                }
            }
        }

        prefixes
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Migration {
    /// Key to migrate from.
    pub from: String,
    /// Key to migrate to.
    pub to: String,
}

impl Schema {
    /// Load schema from the given set of bytes.
    pub fn load_static() -> Result<Schema, Error> {
        Ok(serde_yaml::from_slice(SCHEMA).context("failed to load settings.yaml")?)
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

/// Information on a given prefix.
#[derive(Default)]
struct Prefix {
    /// All keys that belongs to the given prefix.
    keys: Vec<String>,
}

pub struct Inner {
    db: db::Database,
    /// Maps setting prefixes to subscriptions.
    subscriptions: Subscriptions,
    /// Schema for every corresponding type.
    pub schema: Arc<Schema>,
    /// Information about all prefixes.
    prefixes: Arc<HashMap<String, Prefix>>,
}

/// A container for settings from which we can subscribe for updates.
#[derive(Clone)]
pub struct Settings {
    scope: String,
    inner: Arc<Inner>,
}

impl Settings {
    pub fn new(db: db::Database, schema: Schema) -> Self {
        let prefixes = schema.as_prefixes();

        Self {
            scope: String::from(""),
            inner: Arc::new(Inner {
                db,
                subscriptions: Default::default(),
                schema: Arc::new(schema),
                prefixes: Arc::new(prefixes),
            }),
        }
    }

    /// Run all settings migrations.
    pub fn run_migrations(&self) -> Result<(), Error> {
        for m in &self.inner.schema.migrations {
            let from = match self.inner_get::<serde_json::Value>(&m.from)? {
                Some(from) => from,
                None => continue,
            };

            let to = match self.setting::<serde_json::Value>(&m.to)? {
                Some(to) => to,
                None => bail!("No target schema for key: {}", m.to),
            };

            if to.value.is_none() {
                log::info!("Migrating setting: {} -> {}", m.from, m.to);

                if !to.schema.ty.is_compatible_with_json(&from) {
                    bail!(
                        "value for {} (json: {}) is not compatible with {} ({})",
                        m.from,
                        serde_json::to_string(&from)?,
                        m.to,
                        to.schema.ty
                    );
                }

                self.set_json(&m.to, from)?;
            } else {
                log::warn!(
                    "Ignoring value for {} since {} is already present",
                    m.from,
                    m.to
                );
            }

            self.clear(&m.from)?;
        }

        Ok(())
    }

    /// Lookup the given schema.
    pub fn lookup(&self, key: &str) -> Option<&SchemaType> {
        let key = self.key(key);
        self.inner.schema.types.get(key.as_ref())
    }

    /// Get a setting by prefix.
    pub fn list_by_prefix(&self, prefix: &str) -> Result<Vec<Setting>, Error> {
        use self::db::schema::settings::dsl;

        let prefix = self.key(prefix);

        let c = self.inner.db.pool.lock();

        let prefix = match self.inner.prefixes.get(prefix.as_ref()) {
            Some(prefix) => prefix,
            None => return Ok(Vec::default()),
        };

        let mut settings = Vec::new();

        let values = dsl::settings
            .select((dsl::key, dsl::value))
            .order(dsl::key)
            .load::<(String, String)>(&*c)?
            .into_iter()
            .collect::<HashMap<_, _>>();

        for key in &prefix.keys {
            let schema = match self.inner.schema.types.get(key) {
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
    }

    /// Get the given setting.
    ///
    /// This includes the schema of the setting as well.
    pub fn setting<'a, T>(&'a self, key: &str) -> Result<Option<SettingRef<'a, T>>, Error>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let key = self.key(key);

        let schema = match self.inner.schema.types.get(key.as_ref()) {
            Some(schema) => schema,
            None => return Ok(None),
        };

        let value = self.inner_get(&key)?;
        Ok(Some(SettingRef { schema, key, value }))
    }

    /// Test if the given key exists in the database.
    pub fn has(&self, key: &str) -> Result<bool, Error> {
        let key = self.key(key);
        Ok(self.inner_get::<serde_json::Value>(&key)?.is_some())
    }

    /// Get the value of the given key from the database.
    pub fn get<T>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let key = self.key(key);
        self.inner_get(&key)
    }

    /// Insert the given setting without sending an update notification to other components.
    pub fn set_silent<T>(&self, key: &str, value: T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let key = self.key(key);
        self.inner_set(key.as_ref(), value, false)
    }

    /// Insert the given setting.
    pub fn set<T>(&self, key: &str, value: T) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let key = self.key(key);
        self.inner_set(key.as_ref(), value, true)
    }

    /// Insert the given setting as raw JSON.
    pub fn set_json(&self, key: &str, value: serde_json::Value) -> Result<(), Error> {
        let key = self.key(key);
        self.inner_set_json(key.as_ref(), value, true)
    }

    /// Inner implementation of set_json which doesn't do key translation.
    fn inner_set_json(
        &self,
        key: &str,
        value: serde_json::Value,
        notify: bool,
    ) -> Result<(), Error> {
        use self::db::schema::settings::dsl;

        if log::log_enabled!(log::Level::Trace) {
            log::trace!("{}: Setting to {:?} (notify: {})", key, value, notify);
        }

        let c = self.inner.db.pool.lock();

        let filter = dsl::settings.filter(dsl::key.eq(&key));

        let b = filter
            .clone()
            .select((dsl::key, dsl::value))
            .first::<(String, String)>(&*c)
            .optional()?;

        let json = serde_json::to_string(&value)?;

        if notify {
            self.try_send(&key, Event::Set(value));
        }

        match b {
            None => {
                diesel::insert_into(dsl::settings)
                    .values((dsl::key.eq(&key), dsl::value.eq(json)))
                    .execute(&*c)?;
            }
            Some(_) => {
                diesel::update(filter)
                    .set((dsl::key.eq(&key), dsl::value.eq(json)))
                    .execute(&*c)?;
            }
        }

        Ok(())
    }

    /// Insert the given setting.
    pub fn list(&self) -> Result<Vec<Setting>, Error> {
        use self::db::schema::settings::dsl;
        let c = self.inner.db.pool.lock();

        let mut settings = Vec::new();

        let values = dsl::settings
            .select((dsl::key, dsl::value))
            .order(dsl::key)
            .load::<(String, String)>(&*c)?
            .into_iter()
            .collect::<HashMap<_, _>>();

        for (key, schema) in &self.inner.schema.types {
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
    }

    /// Clear the given setting. Returning `true` if it was removed.
    pub fn clear(&self, key: &str) -> Result<bool, Error> {
        use self::db::schema::settings::dsl;

        let key = self.key(key);

        self.try_send(&key, Event::Clear);

        let c = self.inner.db.pool.lock();
        let count = diesel::delete(dsl::settings.filter(dsl::key.eq(key))).execute(&*c)?;
        Ok(count == 1)
    }

    /// Create a scoped setting.
    pub fn scoped(&self, s: &str) -> Settings {
        let mut scope = self.scope.clone();

        for s in s.trim_matches(SEPARATOR).split('/') {
            if s.is_empty() {
                continue;
            }

            if !scope.is_empty() {
                scope.push(SEPARATOR);
            }

            scope.push_str(s);
        }

        Settings {
            scope,
            inner: self.inner.clone(),
        }
    }

    /// Initialize the value from the database.
    pub fn stream<'a, T>(&'a self, key: &str) -> StreamBuilder<'_, T> {
        let key = self.key(key);

        StreamBuilder {
            settings: self,
            default_value: None,
            key,
        }
    }

    /// Get a helper to build synchronized variables.
    pub fn vars(&self) -> Vars {
        Vars {
            settings: self.clone(),
            futures: Vec::new(),
        }
    }

    /// Get the value of the given key from the database.
    fn inner_get<T>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        use self::db::schema::settings::dsl;

        let c = self.inner.db.pool.lock();

        let result = dsl::settings
            .select(dsl::value)
            .filter(dsl::key.eq(&key))
            .first::<String>(&*c)
            .optional()?;

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
    fn inner_set<T>(&self, key: &str, value: T, notify: bool) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let value = serde_json::to_value(value)?;
        self.inner_set_json(key, value, notify)
    }

    /// Subscribe for events on the given key.
    fn make_stream<T>(&self, key: &str, default: T) -> Stream<T>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        Stream {
            default,
            option_stream: self.make_option_stream(key),
        }
    }

    /// Subscribe for any events on the given key.
    fn make_option_stream<T>(&self, key: &str) -> OptionStream<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let (tx, rx) = mpsc::unbounded();

        if !self.inner.schema.contains(key) {
            panic!("no schema registered for key `{}`", key);
        }

        {
            let mut subscriptions = self.inner.subscriptions.write();
            let values = subscriptions.entry(key.to_string()).or_default();

            let mut update = Vec::with_capacity(values.len());

            for tx in values.drain(..) {
                if !tx.is_closed() {
                    update.push(tx);
                }
            }

            update.push(tx);
            *values = update;
        }

        OptionStream {
            key: key.to_string(),
            rx,
            marker: marker::PhantomData,
        }
    }

    /// Try to send the specified event.
    ///
    /// Cleans up the existing subscription if the other side is closed.
    fn try_send(&self, key: &str, event: Event<serde_json::Value>) {
        let subscriptions = self.inner.subscriptions.upgradable_read();

        if let Some(subs) = subscriptions.get(key) {
            log::trace!("{}: Updating {} subscriptions", key, subs.len());

            for sub in subs {
                if let Err(e) = sub.unbounded_send(event.clone()) {
                    log::error!("error when sending to sub: {}: {}", key, e);
                }
            }
        } else {
            log::trace!("{}: No subscription to update", key);
        }
    }

    /// Construct a new key.
    fn key<'a>(&'a self, key: &str) -> Cow<'a, str> {
        let key = key.trim_matches(SEPARATOR);

        if key.is_empty() {
            return Cow::Borrowed(&self.scope);
        }

        if self.scope.is_empty() {
            return Cow::Owned(key.to_string());
        }

        let mut scope = self.scope.clone();
        scope.push(SEPARATOR);
        scope.push_str(key.trim_matches(SEPARATOR));
        Cow::Owned(scope)
    }
}

#[must_use = "Must consume to drive decide how to handle stream"]
pub struct StreamBuilder<'a, T> {
    settings: &'a Settings,
    default_value: Option<T>,
    key: Cow<'a, str>,
}

impl<'a, T> StreamBuilder<'a, T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    /// Make the setting required, falling back to using and storing the default value if necessary.
    pub fn or_default(self) -> Result<(Stream<T>, T), Error>
    where
        T: Clone + Default,
    {
        self.or_with(T::default())
    }

    /// Make the setting required, falling back to using and storing the specified value if necessary.
    pub fn or_with(self, value: T) -> Result<(Stream<T>, T), Error>
    where
        T: Clone,
    {
        self.or_with_else(move || value)
    }

    /// Make the setting required, falling back to using and storing the specified value if necessary.
    pub fn or_with_else<F>(self, value: F) -> Result<(Stream<T>, T), Error>
    where
        T: Clone,
        F: FnOnce() -> T,
    {
        let value = match self.settings.inner_get::<T>(&self.key)? {
            Some(value) => value,
            None => {
                let value = value();
                self.settings.inner_set(&self.key, &value, true)?;
                value
            }
        };

        let stream = self.settings.make_stream(&self.key, value.clone());
        Ok((stream, value))
    }

    /// Make the setting optional.
    pub fn optional(self) -> Result<(OptionStream<T>, Option<T>), Error> {
        let value = self.settings.inner_get::<T>(&self.key)?;

        let value = match value {
            Some(value) => Some(value),
            None => match self.default_value {
                Some(value) => {
                    self.settings.inner_set(&self.key, &value, true)?;
                    Some(value)
                }
                None => None,
            },
        };

        let stream = self.settings.make_option_stream(&self.key);
        Ok((stream, value))
    }

    /// Add a potential fallback value when the type is optional.
    pub fn or(self, other: Option<T>) -> StreamBuilder<'a, T> {
        self.or_else(move || other)
    }

    /// Add a potential fallback value when the type is optional.
    pub fn or_else<F>(mut self, other: F) -> StreamBuilder<'a, T>
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

#[must_use = "Must consume to drive variable updates"]
pub struct Vars {
    settings: Settings,
    futures: Vec<future::BoxFuture<'static, Result<(), Error>>>,
}

impl Vars {
    /// Get a synchronized variable for the given configuration key.
    pub fn var<T>(&mut self, key: &str, default: T) -> Result<Arc<RwLock<T>>, Error>
    where
        T: 'static
            + fmt::Debug
            + Send
            + Sync
            + Clone
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Unpin,
    {
        let (mut stream, value) = self.settings.stream(key).or_with(default)?;
        let value = Arc::new(RwLock::new(value));
        let future_value = value.clone();

        let future = async move {
            while let Some(update) = stream.next().await {
                *future_value.write() = update;
            }

            Ok(())
        };

        self.futures.push(future.boxed());
        Ok(value)
    }

    /// Get an optional synchronized variable for the given configuration key.
    pub fn optional<T>(&mut self, key: &str) -> Result<Arc<RwLock<Option<T>>>, Error>
    where
        T: 'static
            + fmt::Debug
            + Send
            + Sync
            + Clone
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Unpin,
    {
        let (mut stream, value) = self.settings.stream(key).optional()?;
        let value = Arc::new(RwLock::new(value));
        let future_value = value.clone();

        let future = async move {
            while let Some(update) = stream.next().await {
                *future_value.write() = update;
            }

            Ok(())
        };

        self.futures.push(future.boxed());
        Ok(value)
    }

    /// Drive the local variable set.
    pub fn run(self) -> impl Future<Output = Result<(), Error>> {
        let Vars { futures, .. } = self;

        async move {
            let _ = future::try_join_all(futures).await?;
            Ok(())
        }
    }
}

/// Get updates for a specific setting.
pub struct Stream<T> {
    default: T,
    option_stream: OptionStream<T>,
}

impl<T> stream::FusedStream for Stream<T> {
    fn is_terminated(&self) -> bool {
        self.option_stream.is_terminated()
    }
}

impl<T> futures::Stream for Stream<T>
where
    T: fmt::Debug + Unpin + Clone + serde::de::DeserializeOwned,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(
            match ready!(Pin::new(&mut self.option_stream).poll_next(cx)) {
                Some(update) => match update {
                    Some(update) => update,
                    None => self.as_ref().default.clone(),
                },
                None => return Poll::Ready(None),
            },
        ))
    }
}

/// Get updates for a specific setting.
pub struct OptionStream<T> {
    key: String,
    rx: mpsc::UnboundedReceiver<Event<serde_json::Value>>,
    marker: marker::PhantomData<T>,
}

impl<T> stream::FusedStream for OptionStream<T> {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl<T> futures::Stream for OptionStream<T>
where
    T: fmt::Debug + Unpin + serde::de::DeserializeOwned,
{
    type Item = Option<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let update = match ready!(Pin::new(&mut self.rx).poll_next(cx)) {
            Some(update) => update,
            None => return Poll::Ready(None),
        };

        if log::log_enabled!(log::Level::Trace) {
            log::trace!("{}: {:?}", self.as_ref().key, update);
        }

        let value = Some(match update {
            Event::Clear => None,
            Event::Set(value) => match serde_json::from_value(value) {
                Ok(value) => Some(value),
                Err(e) => {
                    log::warn!("bad value for key: {}: {}", self.as_ref().key, e);
                    None
                }
            },
        });

        Poll::Ready(value)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Format {
    /// mysql://<user>:<password>@<host>/<database>
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Type {
    #[serde(default)]
    pub optional: bool,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub title: String,
    pub field: String,
    #[serde(rename = "type")]
    pub ty: Box<Type>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "id")]
pub enum Kind {
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "oauth2-config")]
    Oauth2Config,
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
        options: Vec<serde_json::Value>,
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
            Oauth2Config => serde_json::from_str(s)?,
            Duration => {
                let d = str::parse::<utils::Duration>(s)?;
                Value::String(d.to_string())
            }
            Bool => Value::Bool(str::parse::<bool>(s)?),
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
                            bail!("expected {}", self);
                        }

                        Value::Array(values)
                    }
                    _ => bail!("expected array"),
                }
            }
            Select {
                ref value,
                ref options,
            } => {
                let v = value.parse_as_json(s)?;

                if !options.iter().any(|o| *o == v) {
                    let mut out = Vec::new();

                    for o in options {
                        out.push(serde_json::to_string(o)?);
                    }

                    let alts = out.join(", ");
                    bail!("Expected one of: {}.", alts);
                }

                v
            }
            TimeZone => {
                let tz =
                    str::parse::<Tz>(s).map_err(|s| format_err!("Invalid time zone: {}", s))?;
                Value::String(format!("{:?}", tz))
            }
            Object { ref fields, .. } => {
                let json = serde_json::from_str(s)?;

                let object = match json {
                    Value::Object(object) => object,
                    _ => bail!("Expected a JSON object"),
                };

                for field in fields {
                    let f = match object.get(&field.field) {
                        Some(f) => f,
                        None if field.ty.optional => continue,
                        None => bail!("Missing required field: {}", field.field),
                    };

                    if !field.ty.is_compatible_with_json(f) {
                        bail!("Bad field: {}", field.field);
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
            (Oauth2Config, _) => serde_json::from_value::<oauth2::Config>(other.clone()).is_ok(),
            (Duration, Value::Number(..)) => true,
            (Bool, Value::Bool(..)) => true,
            (Number, Value::Number(..)) => true,
            (Percentage, Value::Number(..)) => true,
            (String { .. }, Value::String(..)) => true,
            (Text, Value::String(..)) => true,
            (Set { ref value }, Value::Array(ref values)) => {
                values.iter().all(|v| value.is_compatible_with_json(v))
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
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Kind::*;

        match &self.kind {
            Raw => write!(fmt, "any")?,
            Oauth2Config => write!(fmt, "secrets")?,
            Duration => write!(fmt, "duration")?,
            Bool => write!(fmt, "bool")?,
            Number => write!(fmt, "number")?,
            Percentage => write!(fmt, "percentage")?,
            String { .. } => write!(fmt, "string")?,
            Text => write!(fmt, "text")?,
            Set { ref value } => write!(fmt, "Array<{}>", value)?,
            Select { ref value, .. } => write!(fmt, "Select<{}>", value)?,
            TimeZone => write!(fmt, "TimeZone")?,
            Object { .. } => write!(fmt, "Object")?,
        };

        if self.optional {
            write!(fmt, "?")?;
        }

        Ok(())
    }
}
