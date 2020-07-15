use crate::db;
use crate::track_id::TrackId;
use crate::utils;
use diesel::prelude::*;
use std::collections::{hash_map, HashMap};
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Local database wrapper.
#[derive(Clone)]
struct Database(db::Database);

impl Database {
    private_database_group_fns!(themes, Theme, Key);

    async fn edit(
        &self,
        key: &Key,
        track_id: &TrackId,
    ) -> Result<Option<db::models::Theme>, anyhow::Error> {
        use db::schema::themes::dsl;

        let key = key.clone();
        let track_id = track_id.clone();

        self.0
            .asyncify(move |c| {
                let filter =
                    dsl::themes.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));

                let first = filter.clone().first::<db::models::Theme>(c).optional()?;

                match first {
                    None => {
                        let theme = db::models::Theme {
                            channel: key.channel.to_string(),
                            name: key.name.to_string(),
                            track_id,
                            start: Default::default(),
                            end: None,
                            group: None,
                            disabled: false,
                        };

                        diesel::insert_into(dsl::themes).values(&theme).execute(c)?;
                        Ok(Some(theme))
                    }
                    Some(theme) => {
                        let mut set = db::models::UpdateTheme::default();
                        set.track_id = Some(&track_id);
                        diesel::update(filter).set(&set).execute(c)?;

                        if theme.disabled {
                            return Ok(None);
                        }

                        Ok(Some(theme))
                    }
                }
            })
            .await
    }

    async fn edit_duration(
        &self,
        key: &Key,
        start: utils::Offset,
        end: Option<utils::Offset>,
    ) -> Result<(), anyhow::Error> {
        use db::schema::themes::dsl;

        let key = key.clone();

        self.0
            .asyncify(move |c| {
                let start = start.as_milliseconds() as i32;
                let end = end.map(|s| s.as_milliseconds() as i32);

                diesel::update(
                    dsl::themes.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
                )
                .set((dsl::start.eq(start), dsl::end.eq(end)))
                .execute(c)?;

                Ok(())
            })
            .await
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
    pub async fn load(db: db::Database) -> Result<Themes, anyhow::Error> {
        let mut inner = HashMap::new();

        let db = Database(db);

        for theme in db.list().await? {
            let theme = Theme::from_db(&theme)?;
            inner.insert(theme.key.clone(), Arc::new(theme));
        }

        Ok(Themes {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub async fn edit(
        &self,
        channel: &str,
        name: &str,
        track_id: TrackId,
    ) -> Result<(), anyhow::Error> {
        let key = Key::new(channel, name);

        let mut inner = self.inner.write().await;

        if let Some(theme) = self.db.edit(&key, &track_id).await? {
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
    pub async fn edit_duration(
        &self,
        channel: &str,
        name: &str,
        start: utils::Offset,
        end: Option<utils::Offset>,
    ) -> Result<(), anyhow::Error> {
        let key = Key::new(channel, name);
        self.db
            .edit_duration(&key, start.clone(), end.clone())
            .await?;

        let mut inner = self.inner.write().await;

        if let hash_map::Entry::Occupied(mut e) = inner.entry(key) {
            let mut update = (**e.get()).clone();
            update.start = start;
            update.end = end;
            e.insert(Arc::new(update));
        }

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
    pub const NAME: &'static str = "theme";

    /// Convert a database theme into an in-memory theme.
    pub fn from_db(theme: &db::models::Theme) -> Result<Theme, anyhow::Error> {
        let key = Key::new(&theme.channel, &theme.name);

        let start = utils::Offset::milliseconds(theme.start as u32);
        let end = theme.end.map(|s| utils::Offset::milliseconds(s as u32));

        Ok(Theme {
            key,
            track_id: theme.track_id.clone(),
            start,
            end,
            group: theme.group.clone(),
            disabled: theme.disabled,
        })
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "track_id = {track_id}, start = {start}, end = {end}, group = {group}, disabled = {disabled}",
            track_id = self.track_id,
            start = self.start,
            end = self
                .end
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_else(|| String::from("*none*")),
            group = self.group.as_deref().unwrap_or("*none*"),
            disabled = self.disabled,
        )
    }
}
