use crate::db;
use crate::template;
use crate::utils;
use diesel::prelude::*;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Local database wrapper.
#[derive(Clone)]
struct Database(db::Database);

impl Database {
    private_database_group_fns!(aliases, Alias, db::Key);

    async fn edit(&self, key: &db::Key, text: &str) -> Result<db::models::Alias, anyhow::Error> {
        use db::schema::aliases::dsl;

        let key = key.clone();
        let text = text.to_string();

        self.0
            .asyncify(move |c| {
                let filter =
                    dsl::aliases.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));

                let first = filter.clone().first::<db::models::Alias>(c).optional()?;

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
                            .execute(c)?;
                        Ok(alias)
                    }
                    Some(alias) => {
                        let mut set = db::models::UpdateAlias::default();
                        set.text = Some(&text);
                        diesel::update(filter).set(&set).execute(c)?;
                        Ok(alias)
                    }
                }
            })
            .await
    }

    /// Edit the pattern of an alias.
    async fn edit_pattern(
        &self,
        key: &db::Key,
        pattern: Option<&regex::Regex>,
    ) -> Result<(), anyhow::Error> {
        use db::schema::aliases::dsl;

        let key = key.clone();
        let pattern = pattern.cloned();

        self.0
            .asyncify(move |c| {
                let pattern = pattern.as_ref().map(|p| p.as_str());

                diesel::update(
                    dsl::aliases.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
                )
                .set(dsl::pattern.eq(pattern))
                .execute(c)?;

                Ok(())
            })
            .await
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
    pub async fn load(db: db::Database) -> Result<Aliases, anyhow::Error> {
        let mut inner = db::Matcher::new();

        let db = Database(db);

        for alias in db.list().await? {
            let alias = Alias::from_db(&alias)?;
            inner.insert(alias.key.clone(), Arc::new(alias));
        }

        Ok(Aliases {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Resolve the given command.
    pub async fn resolve(&self, channel: &str, message: Arc<String>) -> Option<(db::Key, String)> {
        let mut it = utils::Words::new(message);
        let first = it.next();

        if let Some((alias, captures)) =
            self.inner
                .read()
                .await
                .resolve(channel, first.as_deref(), &it)
        {
            let key = alias.key.clone();

            match alias.template.render_to_string(&captures) {
                Ok(s) => return Some((key, s)),
                Err(e) => {
                    log::error!("failed to render alias: {}", e);
                }
            }
        }

        None
    }

    /// Insert a word into the bad words list.
    pub async fn edit(
        &self,
        channel: &str,
        name: &str,
        template: template::Template,
    ) -> Result<(), anyhow::Error> {
        let key = db::Key::new(channel, name);

        let alias = self.db.edit(&key, template.source()).await?;

        if alias.disabled {
            self.inner.write().await.remove(&key);
        } else {
            let pattern = db::Pattern::from_db(alias.pattern.as_ref())?;

            let alias = Alias {
                key: key.clone(),
                pattern,
                template,
                group: alias.group,
                disabled: alias.disabled,
            };

            self.inner.write().await.insert(key, Arc::new(alias));
        }

        Ok(())
    }

    /// Edit the pattern for the given command.
    pub async fn edit_pattern(
        &self,
        channel: &str,
        name: &str,
        pattern: Option<regex::Regex>,
    ) -> Result<bool, anyhow::Error> {
        let key = db::Key::new(channel, name);
        self.db.edit_pattern(&key, pattern.as_ref()).await?;

        Ok(self.inner.write().await.modify(key, |alias| {
            alias.pattern = pattern.map(db::Pattern::regex).unwrap_or_default();
        }))
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
    pub fn from_db(alias: &db::models::Alias) -> Result<Alias, anyhow::Error> {
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
            "template = \"{template}\", pattern = {pattern}, group = {group}, disabled = {disabled}",
            template = self.template,
            pattern = self.pattern,
            group = self.group.as_deref().unwrap_or("*none*"),
            disabled = self.disabled,
        )
    }
}
