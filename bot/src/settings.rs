//! Utilities for dealing with dynamic configuration and settings.

use crate::db;
use diesel::prelude::*;
use futures::{sync::mpsc, Async, Future as _, Poll, Stream as _};
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio_core::reactor::Core;

const SEPARATOR: &'static str = "/";

type EventSender = mpsc::UnboundedSender<Event<serde_json::Value>>;
type Subscriptions = Arc<RwLock<HashMap<String, (Type, EventSender)>>>;

/// Update events for a given key.
#[derive(Clone)]
pub enum Event<T> {
    /// Indicate that the given key was cleared.
    Clear,
    /// Indicate that the given key was updated.
    Set(T),
}

/// A container for settings from which we can subscribe for updates.
#[derive(Clone)]
pub struct Settings {
    db: db::Database,
    /// Maps setting prefixes to subscriptions.
    subscriptions: Subscriptions,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Setting {
    #[serde(rename = "type")]
    ty: Option<Type>,
    key: String,
    value: serde_json::Value,
}

impl Settings {
    pub fn new(db: db::Database) -> Self {
        Self {
            db,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
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

            if let Some((_, sub)) = subscriptions.get(key) {
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
        let subscriptions = self.subscriptions.read();

        for (key, value) in dsl::settings
            .select((dsl::key, dsl::value))
            .order(dsl::key)
            .load::<(String, String)>(&*c)?
        {
            let value = serde_json::from_str(&value)?;

            let ty = match subscriptions.get(&key) {
                Some((ty, _)) => Some(ty.clone()),
                None => None,
            };

            settings.push(Setting {
                ty,
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

            if let Some((_, sub)) = subscriptions.get(key) {
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
    pub fn stream<T>(&self, key: &str, default: T, ty: Type) -> Stream<T>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        let (tx, rx) = mpsc::unbounded();

        let mut subscriptions = self.subscriptions.write();

        if subscriptions.insert(key.to_string(), (ty, tx)).is_some() {
            panic!("already a subscription for key: {}", key);
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
        ty: Type,
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

        Ok((self.stream(key, default, ty), value))
    }

    /// Get a synchronized variable for the given configuration key.
    pub fn sync_var<T>(
        &self,
        core: &mut Core,
        key: &str,
        default: T,
        ty: Type,
    ) -> Result<Arc<RwLock<T>>, failure::Error>
    where
        T: 'static + Send + Sync + Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        let (stream, value) = self.init_and_stream(key, default, ty)?;

        let value = Arc::new(RwLock::new(value));

        let future = stream.for_each({
            let value = value.clone();

            move |update| {
                *value.write() = update;
            }
        });

        core.runtime().executor().spawn(future.map_err(|e| {
            log::error!("sync_var update future failed: {}", e);
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
    pub fn stream<T>(&self, key: &str, default: T, ty: Type) -> Stream<T>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        self.settings.stream(&self.scope(key), default, ty)
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
        ty: Type,
    ) -> Result<(Stream<T>, T), failure::Error>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        self.settings.init_and_stream(&self.scope(key), default, ty)
    }

    /// Get a synchronized variable for the given configuration key.
    pub fn sync_var<T>(
        &self,
        core: &mut Core,
        key: &str,
        default: T,
        ty: Type,
    ) -> Result<Arc<RwLock<T>>, failure::Error>
    where
        T: 'static + Send + Sync + Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        self.settings.sync_var(core, &self.scope(key), default, ty)
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

/// A future that calls a function for each settings update.
pub struct ForEach<T, F> {
    stream: Stream<T>,
    f: F,
}

impl<T, F> futures::Future for ForEach<T, F>
where
    F: Fn(T),
    T: Clone + serde::de::DeserializeOwned,
{
    type Item = ();
    type Error = StreamError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        (self.f)(futures::try_ready!(self.stream.poll()));
        Ok(Async::NotReady)
    }
}

impl<T> Stream<T> {
    /// Convert the stream into a future that is driven to completion, calling the given function for each value.
    fn for_each<F>(self, f: F) -> ForEach<T, F>
    where
        F: Fn(T),
    {
        ForEach { stream: self, f: f }
    }
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

impl<T> futures::Future for Stream<T>
where
    T: Clone + serde::de::DeserializeOwned,
{
    type Item = T;
    type Error = StreamError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
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

            return Ok(Async::Ready(value));
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "id")]
pub enum Type {
    #[serde(rename = "duration")]
    Duration,
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "number")]
    U32,
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
}
