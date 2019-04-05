use diesel::prelude::*;
use futures::{sync::mpsc, Async, Poll};
use hashbrown::{hash_map, HashMap};
use parking_lot::RwLock;
use std::{
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

const SEPARATOR: &'static str = "/";

/// Update events for a given key.
#[derive(Clone)]
pub enum Event<T> {
    /// Indicate that the given key was cleared.
    Clear,
    /// Indicate that the given key was updated.
    Set(T),
}

type SubId = usize;
type Subscriptions = HashMap<SubId, mpsc::UnboundedSender<Event<String>>>;

/// A container for settings from which we can subscribe for updates.
#[derive(Clone)]
pub struct Settings {
    db: super::Database,
    /// Maps setting prefixes to subscriptions.
    subscriptions: Arc<RwLock<HashMap<String, Subscriptions>>>,
    id_gen: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Setting {
    key: String,
    value: String,
}

impl Settings {
    pub fn new(db: super::Database) -> Self {
        Self {
            db,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            id_gen: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get the value of the given key from the database.
    pub fn get<T>(&self, key: &str) -> Result<Option<T>, failure::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        use super::schema::settings::dsl;
        let c = self.db.pool.get()?;

        let result = dsl::settings
            .select(dsl::value)
            .filter(dsl::key.eq(key))
            .first::<String>(&c)
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
    pub fn set<T>(&self, key: &str, value: &T) -> Result<(), failure::Error>
    where
        T: serde::Serialize,
    {
        let value = serde_json::to_string(value)?;
        self.set_json(key, value)
    }

    /// Insert the given setting as raw JSON.
    pub fn set_json(&self, key: &str, value: String) -> Result<(), failure::Error> {
        use super::schema::settings::dsl;

        {
            let subscriptions = self.subscriptions.read();

            if let Some(subs) = subscriptions.get(key) {
                for (id, sub) in subs {
                    if let Err(_) = sub.unbounded_send(Event::Set(value.clone())) {
                        log::error!("failed to send message to subscription: {}", id);
                    }
                }
            }
        }

        let c = self.db.pool.get()?;

        let filter = dsl::settings.filter(dsl::key.eq(&key));
        let b = filter
            .clone()
            .select((dsl::key, dsl::value))
            .first::<(String, String)>(&c)
            .optional()?;

        match b {
            None => {
                diesel::insert_into(dsl::settings)
                    .values((dsl::key.eq(key), dsl::value.eq(value)))
                    .execute(&c)?;
            }
            Some(_) => {
                diesel::update(filter)
                    .set((dsl::key.eq(key), dsl::value.eq(&value)))
                    .execute(&c)?;
            }
        }

        Ok(())
    }

    /// Insert the given setting.
    pub fn list(&self) -> Result<Vec<Setting>, failure::Error> {
        use super::schema::settings::dsl;
        let c = self.db.pool.get()?;

        let mut settings = Vec::new();

        for (key, value) in dsl::settings
            .select((dsl::key, dsl::value))
            .order(dsl::key)
            .load::<(String, String)>(&c)?
        {
            settings.push(Setting {
                key: key.to_string(),
                value: value.to_string(),
            });
        }

        Ok(settings)
    }

    /// Clear the given setting. Returning `true` if it was removed.
    pub fn clear(&self, key: &str) -> Result<bool, failure::Error> {
        {
            let subscriptions = self.subscriptions.read();

            if let Some(subs) = subscriptions.get(key) {
                for (id, sub) in subs {
                    if let Err(_) = sub.unbounded_send(Event::Clear) {
                        log::error!("failed to send message to subscription: {}", id);
                    }
                }
            }
        }

        use super::schema::settings::dsl;
        let c = self.db.pool.get()?;
        let count = diesel::delete(dsl::settings.filter(dsl::key.eq(key))).execute(&c)?;
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
        T: Clone,
    {
        let id = self.id_gen.fetch_add(1, Ordering::SeqCst);

        let (tx, rx) = mpsc::unbounded();

        let mut subscriptions = self.subscriptions.write();

        let m = match subscriptions.entry(key.to_string()) {
            hash_map::Entry::Vacant(e) => e.insert(Default::default()),
            hash_map::Entry::Occupied(e) => e.into_mut(),
        };

        m.insert(id, tx);

        Stream {
            default,
            settings: self.clone(),
            id,
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
        T: serde::de::DeserializeOwned,
    {
        self.settings.get(&self.scope(key))
    }

    /// Insert the given setting.
    pub fn set<T>(&self, key: &str, value: &T) -> Result<(), failure::Error>
    where
        T: serde::Serialize,
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
        T: Clone,
    {
        self.settings.stream(&self.scope(key), default)
    }

    fn scope(&self, key: &str) -> String {
        let mut scope = self.scope.clone();
        scope.push(key.to_string());
        scope.join(SEPARATOR)
    }
}

/// Get updates for a specific setting.
pub struct Stream<T> {
    default: T,
    settings: Settings,
    id: SubId,
    key: String,
    rx: mpsc::UnboundedReceiver<Event<String>>,
}

impl<T> Drop for Stream<T> {
    fn drop(&mut self) {
        let mut subscriptions = self.settings.subscriptions.write();

        if let Some(subs) = subscriptions.get_mut(&self.key) {
            if let Some(_) = subs.remove(&self.id) {
                return;
            }
        }

        log::warn!("Subscription dropped, but failed to clean up Settings");
    }
}

impl<T> futures::Stream for Stream<T>
where
    T: Clone + fmt::Debug + serde::de::DeserializeOwned,
{
    type Item = T;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let n = match futures::try_ready!(self.rx.poll()) {
                Some(e) => match e {
                    Event::Clear => Some(self.default.clone()),
                    Event::Set(value) => {
                        let value = match serde_json::from_str(&value) {
                            Ok(value) => value,
                            Err(e) => {
                                log::warn!("bad value for key: {}: {}", self.key, e);
                                continue;
                            }
                        };

                        Some(value)
                    }
                },
                None => None,
            };

            return Ok(Async::Ready(n));
        }
    }
}
