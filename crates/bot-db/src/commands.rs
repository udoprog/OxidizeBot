use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Context, Error, Result};
use common::words;
use common::Channel;
use diesel::prelude::*;
use serde::{ser, Serialize};
use tokio::sync::RwLock;

/// Local database wrapper.
#[derive(Clone)]
struct Database(crate::Database);

impl Database {
    private_database_group_fns!(commands, Command, crate::Key);

    /// Edit the text for the given key.
    async fn edit(&self, key: &crate::Key, text: &str) -> Result<crate::models::Command, Error> {
        use crate::schema::commands::dsl;

        let key = key.clone();
        let text = text.to_string();

        self.0
            .asyncify(move |c| {
                let filter = dsl::commands
                    .filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));

                match filter.first::<crate::models::Command>(c).optional()? {
                    None => {
                        let command = crate::models::Command {
                            channel: key.channel.to_owned(),
                            pattern: None,
                            name: key.name.to_string(),
                            count: 0,
                            text: text.to_string(),
                            group: None,
                            disabled: false,
                        };

                        diesel::insert_into(dsl::commands)
                            .values(&command)
                            .execute(c)?;
                        Ok(command)
                    }
                    Some(command) => {
                        let mut set = crate::models::UpdateCommand::default();
                        set.text = Some(&text);
                        diesel::update(filter).set(&set).execute(c)?;
                        Ok(command)
                    }
                }
            })
            .await
    }

    /// Edit the pattern of a command.
    async fn edit_pattern(&self, key: &crate::Key, pattern: Option<&regex::Regex>) -> Result<()> {
        use crate::schema::commands::dsl;

        let key = key.clone();
        let pattern = pattern.cloned();

        self.0
            .asyncify(move |c| {
                let pattern = pattern.as_ref().map(|p| p.as_str());

                diesel::update(
                    dsl::commands
                        .filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
                )
                .set(dsl::pattern.eq(pattern))
                .execute(c)?;

                Ok(())
            })
            .await
    }

    /// Increment the given key.
    async fn increment(&self, key: &crate::Key) -> Result<bool, Error> {
        use crate::schema::commands::dsl;

        let key = key.clone();

        self.0
            .asyncify(move |c| {
                let count = diesel::update(
                    dsl::commands
                        .filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
                )
                .set(dsl::count.eq(dsl::count + 1))
                .execute(c)?;
                Ok(count == 1)
            })
            .await
    }
}

#[derive(Clone)]
pub struct Commands {
    inner: Arc<RwLock<crate::Matcher<Command>>>,
    db: Database,
}

impl Commands {
    database_group_fns!(Command, crate::Key);

    /// Construct a new commands store with a db.
    pub async fn load(db: crate::Database) -> Result<Commands, Error> {
        let db = Database(db);

        let mut matcher = crate::Matcher::new();

        for command in db.list().await? {
            let command = Command::from_db(&command)?;
            matcher.insert(command.key.clone(), Arc::new(command));
        }

        Ok(Commands {
            inner: Arc::new(RwLock::new(matcher)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub async fn edit(
        &self,
        channel: &Channel,
        name: &str,
        template: template::Template,
    ) -> Result<(), Error> {
        let key = crate::Key::new(channel, name);

        let mut inner = self.inner.write().await;
        let command = self.db.edit(&key, template.source()).await?;

        if command.disabled {
            inner.remove(&key);
        } else {
            let vars = template.vars();

            let command = Arc::new(Command {
                key: key.clone(),
                pattern: crate::Pattern::from_db(command.pattern.as_ref())?,
                count: Arc::new(AtomicUsize::new(command.count as usize)),
                template,
                vars,
                group: command.group,
                disabled: command.disabled,
            });

            inner.insert(key, command);
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

        Ok(self.inner.write().await.modify(key, |command| {
            command.pattern = pattern.map(crate::Pattern::regex).unwrap_or_default();
        }))
    }

    /// Increment the specified command.
    pub async fn increment(&self, command: &Command) -> Result<(), Error> {
        self.db.increment(&command.key).await?;
        command.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Resolve the given command.
    pub async fn resolve<'a>(
        &self,
        channel: &'a Channel,
        first: Option<&'a str>,
        it: &'a words::Split,
    ) -> Option<(Arc<Command>, crate::Captures<'a>)> {
        let inner = self.inner.read().await;

        inner
            .resolve(channel, first, it)
            .map(|(command, captures)| (command.clone(), captures))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Command {
    /// Key for the command.
    pub key: crate::Key,
    /// Pattern to use for matching command.
    pub pattern: crate::Pattern,
    /// Count associated with the command.
    #[serde(serialize_with = "serialize_count")]
    count: Arc<AtomicUsize>,
    pub template: template::Template,
    vars: HashSet<String>,
    pub group: Option<String>,
    pub disabled: bool,
}

/// Serialize the atomic count.
fn serialize_count<S>(value: &Arc<AtomicUsize>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    use Serialize as _;

    value
        .load(std::sync::atomic::Ordering::SeqCst)
        .serialize(serializer)
}

impl Command {
    pub(crate) const NAME: &'static str = "command";

    /// Load a command from the database.
    pub(crate) fn from_db(command: &crate::models::Command) -> Result<Command, Error> {
        let template = template::Template::compile(&command.text)
            .with_context(|| anyhow!("failed to compile command `{:?}` from db", command))?;

        let key = crate::Key::new(&command.channel, &command.name);
        let count = Arc::new(AtomicUsize::new(command.count as usize));
        let vars = template.vars();

        let pattern = crate::Pattern::from_db(command.pattern.as_ref())?;

        Ok(Command {
            key,
            pattern,
            count,
            template,
            vars,
            group: command.group.clone(),
            disabled: command.disabled,
        })
    }

    /// Get the current count.
    pub fn count(&self) -> i32 {
        self.count.load(Ordering::SeqCst) as i32
    }

    /// Render the given command.
    pub fn render<T>(&self, data: &T) -> Result<String, Error>
    where
        T: Serialize,
    {
        self.template.render_to_string(data)
    }

    /// Test if the rendered command has the given var.
    pub fn has_var(&self, var: &str) -> bool {
        self.vars.contains(var)
    }
}

impl crate::Matchable for Command {
    fn key(&self) -> &crate::Key {
        &self.key
    }

    fn pattern(&self) -> &crate::Pattern {
        &self.pattern
    }
}

impl fmt::Display for Command {
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
