use crate::{db, template, utils};
use diesel::prelude::*;
use hashbrown::HashMap;
use parking_lot::RwLock;
use std::{fmt, sync::Arc};

/// Local database wrapper.
#[derive(Clone)]
struct Database(db::Database);

impl Database {
    private_database_group_fns!(aliases, Alias, Key);

    fn edit(&self, key: &Key, text: &str) -> Result<Option<db::models::Alias>, failure::Error> {
        use db::schema::aliases::dsl;
        let c = self.0.pool.lock();

        let filter =
            dsl::aliases.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));

        let first = filter.clone().first::<db::models::Alias>(&*c).optional()?;

        match first {
            None => {
                let alias = db::models::Alias {
                    channel: key.channel.to_string(),
                    name: key.name.to_string(),
                    text: text.to_string(),
                    group: None,
                    disabled: false,
                };

                diesel::insert_into(dsl::aliases)
                    .values(&alias)
                    .execute(&*c)?;
                Ok(Some(alias))
            }
            Some(alias) => {
                let mut set = db::models::UpdateAlias::default();
                set.text = Some(text);
                diesel::update(filter).set(&set).execute(&*c)?;

                if alias.disabled {
                    return Ok(None);
                }

                Ok(Some(alias))
            }
        }
    }
}

#[derive(Clone)]
pub struct Aliases {
    inner: Arc<RwLock<HashMap<Key, Arc<Alias>>>>,
    db: Database,
}

impl Aliases {
    database_group_fns!(Alias, Key);

    /// Construct a new commands store with a db.
    pub fn load(db: db::Database) -> Result<Aliases, failure::Error> {
        let mut inner = HashMap::new();

        let db = Database(db);

        for alias in db.list()? {
            let alias = Alias::from_db(alias)?;
            inner.insert(alias.key.clone(), Arc::new(alias));
        }

        Ok(Aliases {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Lookup an alias based on a command prefix.
    pub fn lookup<'a>(&self, channel: &str, it: utils::Words<'a>) -> Option<(&'a str, String)> {
        let it = it.into_iter();

        let inner = self.inner.read();

        for (key, alias) in inner.iter() {
            if key.channel != channel {
                continue;
            }

            if let Some((m, out)) = alias.matches(it.clone()) {
                return Some((m, out));
            }
        }

        None
    }

    /// Insert a word into the bad words list.
    pub fn edit(
        &self,
        channel: &str,
        name: &str,
        template: template::Template,
    ) -> Result<(), failure::Error> {
        let key = Key::new(channel, name);

        let mut inner = self.inner.write();

        if let Some(alias) = self.db.edit(&key, template.source())? {
            log::info!("inserting alias in-memory");

            inner.insert(
                key.clone(),
                Arc::new(Alias {
                    key,
                    template,
                    group: alias.group,
                    disabled: alias.disabled,
                }),
            );
        } else {
            inner.remove(&key);
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
pub struct Alias {
    pub key: Key,
    pub template: template::Template,
    pub group: Option<String>,
    pub disabled: bool,
}

impl Alias {
    pub const NAME: &'static str = "alias";

    /// Convert a database alias into an in-memory alias.
    pub fn from_db(alias: db::models::Alias) -> Result<Alias, failure::Error> {
        let key = Key::new(alias.channel.as_str(), alias.name.as_str());
        let template = template::Template::compile(alias.text)?;

        Ok(Alias {
            key,
            template,
            group: alias.group,
            disabled: alias.disabled,
        })
    }

    /// Test if the given input matches and return the corresonding replacement if it does.
    pub fn matches<'a>(&self, mut it: utils::Words<'a>) -> Option<(&'a str, String)> {
        match it.next() {
            Some(value) if value.to_lowercase() == self.key.name => {
                let data = Data { rest: it.rest() };

                match self.template.render_to_string(&data) {
                    Ok(s) => return Some((value, s)),
                    Err(e) => {
                        log::error!("failed to render alias: {}", e);
                    }
                }
            }
            _ => {}
        }

        return None;

        #[derive(serde::Serialize)]
        struct Data<'a> {
            rest: &'a str,
        }
    }
}

impl fmt::Display for Alias {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "template = \"{template}\", group = {group}, disabled = {disabled}",
            template = self.template,
            group = self.group.as_ref().map(|g| g.as_str()).unwrap_or("*none*"),
            disabled = self.disabled,
        )
    }
}
