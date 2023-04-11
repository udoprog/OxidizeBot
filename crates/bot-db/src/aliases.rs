use std::fmt;
use std::sync::Arc;

use anyhow::Result;
use common::Channel;
use diesel::prelude::*;
use tokio::sync::RwLock;

/// Local database wrapper.
#[derive(Clone)]
struct Database(crate::Database);

impl Database {
    private_database_group_fns!(aliases, Alias, crate::Key);

    async fn edit(&self, key: &crate::Key, text: &str) -> Result<crate::models::Alias> {
        use crate::schema::aliases::dsl;

        let key = key.clone();
        let text = text.to_string();

        self.0
            .asyncify(move |c| {
                let filter =
                    dsl::aliases.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));

                let first = filter.first::<crate::models::Alias>(c).optional()?;

                match first {
                    None => {
                        let alias = crate::models::Alias {
                            channel: key.channel.clone(),
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
                        let mut set = crate::models::UpdateAlias::default();
                        set.text = Some(&text);
                        diesel::update(filter).set(&set).execute(c)?;
                        Ok(alias)
                    }
                }
            })
            .await
    }

    /// Edit the pattern of an alias.
    async fn edit_pattern(&self, key: &crate::Key, pattern: Option<&regex::Regex>) -> Result<()> {
        use crate::schema::aliases::dsl;

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
    inner: Arc<RwLock<crate::Matcher<Alias>>>,
    db: Database,
}

impl Aliases {
    database_group_fns!(Alias, crate::Key);

    /// Construct a new commands store with a db.
    pub async fn load(db: crate::Database) -> Result<Aliases> {
        let mut inner = crate::Matcher::new();

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
    pub async fn resolve(
        &self,
        channel: &Channel,
        message: Arc<String>,
    ) -> Option<(crate::Key, String)> {
        let mut it = common::words::split(message);
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
                    tracing::error!("Failed to render alias: {}", e);
                }
            }
        }

        None
    }

    /// Insert a word into the bad words list.
    pub async fn edit(
        &self,
        channel: &Channel,
        name: &str,
        template: template::Template,
    ) -> Result<()> {
        let key = crate::Key::new(channel, name);

        let alias = self.db.edit(&key, template.source()).await?;

        if alias.disabled {
            self.inner.write().await.remove(&key);
        } else {
            let pattern = crate::Pattern::from_db(alias.pattern.as_ref())?;

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
        channel: &Channel,
        name: &str,
        pattern: Option<regex::Regex>,
    ) -> Result<bool> {
        let key = crate::Key::new(channel, name);
        self.db.edit_pattern(&key, pattern.as_ref()).await?;

        Ok(self.inner.write().await.modify(key, |alias| {
            alias.pattern = pattern.map(crate::Pattern::regex).unwrap_or_default();
        }))
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Alias {
    pub key: crate::Key,
    pub pattern: crate::Pattern,
    pub template: template::Template,
    pub group: Option<String>,
    pub disabled: bool,
}

impl crate::Matchable for Alias {
    fn key(&self) -> &crate::Key {
        &self.key
    }

    fn pattern(&self) -> &crate::Pattern {
        &self.pattern
    }
}

impl Alias {
    pub(crate) const NAME: &'static str = "alias";

    /// Convert a database alias into an in-memory alias.
    pub(crate) fn from_db(alias: &crate::models::Alias) -> Result<Alias> {
        let key = crate::Key::new(&alias.channel, &alias.name);
        let pattern = crate::Pattern::from_db(alias.pattern.as_ref())?;
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
