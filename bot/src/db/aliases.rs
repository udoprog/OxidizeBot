use crate::{db, template, utils};
use diesel::prelude::*;
use parking_lot::RwLock;
use std::{fmt, sync::Arc};

/// Local database wrapper.
#[derive(Clone)]
struct Database(db::Database);

impl Database {
    private_database_group_fns!(aliases, Alias, db::Key);

    fn edit(&self, key: &db::Key, text: &str) -> Result<db::models::Alias, failure::Error> {
        use db::schema::aliases::dsl;
        let c = self.0.pool.lock();

        let filter =
            dsl::aliases.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));

        let first = filter.clone().first::<db::models::Alias>(&*c).optional()?;

        match first {
            None => {
                let alias = db::models::Alias {
                    channel: key.channel.to_string(),
                    pattern: None,
                    name: key.name.to_string(),
                    text: text.to_string(),
                    group: None,
                    disabled: false,
                };

                diesel::insert_into(dsl::aliases)
                    .values(&alias)
                    .execute(&*c)?;
                Ok(alias)
            }
            Some(alias) => {
                let mut set = db::models::UpdateAlias::default();
                set.text = Some(text);
                diesel::update(filter).set(&set).execute(&*c)?;
                Ok(alias)
            }
        }
    }

    /// Edit the pattern of an alias.
    fn edit_pattern(
        &self,
        key: &db::Key,
        pattern: Option<&regex::Regex>,
    ) -> Result<(), failure::Error> {
        use db::schema::aliases::dsl;
        let c = self.0.pool.lock();

        let pattern = pattern.map(|p| p.as_str());

        diesel::update(
            dsl::aliases.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
        )
        .set(dsl::pattern.eq(pattern))
        .execute(&*c)?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct Aliases {
    inner: Arc<RwLock<db::Matcher<Alias>>>,
    db: Database,
}

impl Aliases {
    database_group_fns!(Alias, db::Key);

    /// Construct a new commands store with a db.
    pub fn load(db: db::Database) -> Result<Aliases, failure::Error> {
        let mut inner = db::Matcher::new();

        let db = Database(db);

        for alias in db.list()? {
            let alias = Alias::from_db(&alias)?;
            inner.insert(alias.key.clone(), Arc::new(alias));
        }

        Ok(Aliases {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Resolve the given command.
    pub fn resolve<'a>(
        &self,
        channel: &str,
        first: Option<&str>,
        it: &utils::Words<'a>,
    ) -> Option<String> {
        if let Some((alias, captures)) = self.inner.read().resolve(channel, first, it) {
            match alias.template.render_to_string(&captures) {
                Ok(s) => return Some(s),
                Err(e) => {
                    log::error!("failed to render alias: {}", e);
                }
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
        let key = db::Key::new(channel, name);

        let alias = self.db.edit(&key, template.source())?;

        if alias.disabled {
            self.inner.write().remove(&key);
        } else {
            let pattern = db::Pattern::from_db(alias.pattern.as_ref())?;

            let alias = Alias {
                key: key.clone(),
                pattern,
                template,
                group: alias.group,
                disabled: alias.disabled,
            };

            self.inner.write().insert(key, Arc::new(alias));
        }

        Ok(())
    }

    /// Edit the pattern for the given command.
    pub fn edit_pattern(
        &self,
        channel: &str,
        name: &str,
        pattern: Option<regex::Regex>,
    ) -> Result<(), failure::Error> {
        let key = db::Key::new(channel, name);
        self.db.edit_pattern(&key, pattern.as_ref())?;

        self.inner
            .write()
            .modify_with_pattern(key, pattern, |alias, pattern| {
                alias.pattern = pattern;
            });

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Alias {
    pub key: db::Key,
    pub pattern: db::Pattern,
    pub template: template::Template,
    pub group: Option<String>,
    pub disabled: bool,
}

impl db::Matchable for Alias {
    fn key(&self) -> &db::Key {
        &self.key
    }

    fn pattern(&self) -> &db::Pattern {
        &self.pattern
    }
}

impl Alias {
    pub const NAME: &'static str = "alias";

    /// Convert a database alias into an in-memory alias.
    pub fn from_db(alias: &db::models::Alias) -> Result<Alias, failure::Error> {
        let key = db::Key::new(&alias.channel, &alias.name);
        let pattern = db::Pattern::from_db(alias.pattern.as_ref())?;
        let template = template::Template::compile(&alias.text)?;

        Ok(Alias {
            key,
            pattern,
            template,
            group: alias.group.clone(),
            disabled: alias.disabled,
        })
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
