use crate::{db, track_id::TrackId, utils};
use diesel::prelude::*;
use hashbrown::{hash_map, HashMap};
use parking_lot::RwLock;
use std::sync::Arc;

/// Local database wrapper.
#[derive(Clone)]
struct Database(db::Database);

impl Database {
    private_database_group_fns!(themes, Theme, Key);

    fn edit(
        &self,
        key: &Key,
        track_id: &TrackId,
    ) -> Result<Option<db::models::Theme>, failure::Error> {
        use db::schema::themes::dsl;
        let c = self.0.pool.lock();

        let filter = dsl::themes.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));

        let first = filter.clone().first::<db::models::Theme>(&*c).optional()?;

        match first {
            None => {
                let theme = db::models::Theme {
                    channel: key.channel.to_string(),
                    name: key.name.to_string(),
                    track_id: track_id.clone(),
                    start: Default::default(),
                    end: None,
                    group: None,
                    disabled: false,
                };

                diesel::insert_into(dsl::themes)
                    .values(&theme)
                    .execute(&*c)?;
                Ok(Some(theme))
            }
            Some(theme) => {
                let mut set = db::models::UpdateTheme::default();
                set.track_id = Some(track_id);
                diesel::update(filter).set(&set).execute(&*c)?;

                if theme.disabled {
                    return Ok(None);
                }

                Ok(Some(theme))
            }
        }
    }

    fn edit_duration(
        &self,
        key: &Key,
        start: utils::Offset,
        end: Option<utils::Offset>,
    ) -> Result<(), failure::Error> {
        use db::schema::themes::dsl;
        let c = self.0.pool.lock();

        let start = start.as_milliseconds() as i32;
        let end = end.map(|s| s.as_milliseconds() as i32);

        diesel::update(
            dsl::themes.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
        )
        .set((dsl::start.eq(start), dsl::end.eq(end)))
        .execute(&*c)?;

        Ok(())
    }

    fn delete(&self, key: &Key) -> Result<bool, failure::Error> {
        use db::schema::themes::dsl;

        let c = self.0.pool.lock();
        let count = diesel::delete(
            dsl::themes.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
        )
        .execute(&*c)?;

        Ok(count == 1)
    }

    fn rename(&self, from: &Key, to: &Key) -> Result<bool, failure::Error> {
        use db::schema::themes::dsl;

        let c = self.0.pool.lock();
        let count = diesel::update(
            dsl::themes.filter(dsl::channel.eq(&from.channel).and(dsl::name.eq(&from.name))),
        )
        .set((dsl::name.eq(&to.name), dsl::name.eq(&to.channel)))
        .execute(&*c)?;

        Ok(count == 1)
    }
}

#[derive(Clone)]
pub struct Themes {
    inner: Arc<RwLock<HashMap<Key, Arc<Theme>>>>,
    db: Database,
}

impl Themes {
    database_group_fns!(Theme, Key);

    /// Construct a new commands store with a db.
    pub fn load(db: db::Database) -> Result<Themes, failure::Error> {
        let mut inner = HashMap::new();

        let db = Database(db);

        for theme in db.list()? {
            let theme = Theme::from_db(theme)?;
            inner.insert(theme.key.clone(), Arc::new(theme));
        }

        Ok(Themes {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub fn edit(&self, channel: &str, name: &str, track_id: TrackId) -> Result<(), failure::Error> {
        let key = Key::new(channel, name);

        let mut inner = self.inner.write();

        if let Some(theme) = self.db.edit(&key, &track_id)? {
            log::info!("inserting theme in-memory");

            let start = utils::Offset::milliseconds(theme.start as u32);
            let end = theme.end.map(|s| utils::Offset::milliseconds(s as u32));

            inner.insert(
                key.clone(),
                Arc::new(Theme {
                    key,
                    track_id,
                    start,
                    end,
                    group: theme.group,
                    disabled: theme.disabled,
                }),
            );
        } else {
            inner.remove(&key);
        }

        Ok(())
    }

    /// Edit the duration of the given theme.
    pub fn edit_duration(
        &self,
        channel: &str,
        name: &str,
        start: utils::Offset,
        end: Option<utils::Offset>,
    ) -> Result<(), failure::Error> {
        let key = Key::new(channel, name);
        self.db.edit_duration(&key, start.clone(), end.clone())?;

        let mut inner = self.inner.write();

        if let hash_map::Entry::Occupied(mut e) = inner.entry(key) {
            let mut update = (**e.get()).clone();
            update.start = start;
            update.end = end;
            e.insert(Arc::new(update));
        }

        Ok(())
    }

    /// Remove theme.
    pub fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        let key = Key::new(channel, name);

        if !self.db.delete(&key)? {
            return Ok(false);
        }

        self.inner.write().remove(&key);
        Ok(true)
    }

    /// Get the given theme.
    pub fn get<'a>(&'a self, channel: &str, name: &str) -> Option<Arc<Theme>> {
        let key = Key::new(channel, name);

        let inner = self.inner.read();

        if let Some(theme) = inner.get(&key) {
            return Some(Arc::clone(theme));
        }

        None
    }

    /// Get a list of all commands.
    pub fn list(&self, channel: &str) -> Vec<Arc<Theme>> {
        let inner = self.inner.read();

        let mut out = Vec::new();

        for c in inner.values() {
            if c.key.channel != channel {
                continue;
            }

            out.push(Arc::clone(c));
        }

        out
    }

    /// Try to rename the theme.
    pub fn rename(&self, channel: &str, from: &str, to: &str) -> Result<(), super::RenameError> {
        let from_key = Key::new(channel, from);
        let to_key = Key::new(channel, to);

        let mut inner = self.inner.write();

        if inner.contains_key(&to_key) {
            return Err(super::RenameError::Conflict);
        }

        let theme = match inner.remove(&from_key) {
            Some(theme) => theme,
            None => return Err(super::RenameError::Missing),
        };

        let mut theme = (*theme).clone();
        theme.key = to_key.clone();

        match self.db.rename(&from_key, &to_key) {
            Err(e) => {
                log::error!("failed to rename theme `{}` in database: {}", from, e);
            }
            Ok(false) => {
                log::warn!("theme {} not renamed in database", from);
            }
            Ok(true) => (),
        }

        inner.insert(to_key, Arc::new(theme));
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub struct Key {
    pub channel: String,
    pub name: String,
}

impl Key {
    pub fn new(channel: &str, name: &str) -> Self {
        Self {
            channel: channel.to_string(),
            name: name.to_lowercase(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Theme {
    pub key: Key,
    pub track_id: TrackId,
    pub start: utils::Offset,
    pub end: Option<utils::Offset>,
    pub group: Option<String>,
    pub disabled: bool,
}

impl Theme {
    /// Convert a database theme into an in-memory theme.
    pub fn from_db(theme: db::models::Theme) -> Result<Theme, failure::Error> {
        let key = Key::new(theme.channel.as_str(), theme.name.as_str());

        let start = utils::Offset::milliseconds(theme.start as u32);
        let end = theme.end.map(|s| utils::Offset::milliseconds(s as u32));

        Ok(Theme {
            key,
            track_id: theme.track_id,
            start,
            end,
            group: theme.group,
            disabled: theme.disabled,
        })
    }
}
