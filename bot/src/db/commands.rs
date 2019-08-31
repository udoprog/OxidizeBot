use crate::{db, template, utils};
use diesel::prelude::*;
use failure::{format_err, Error, ResultExt as _};
use hashbrown::HashSet;
use parking_lot::RwLock;
use std::{
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

/// Local database wrapper.
#[derive(Clone)]
struct Database(db::Database);

impl Database {
    private_database_group_fns!(commands, Command, db::Key);

    /// Edit the text for the given key.
    fn edit(&self, key: &db::Key, text: &str) -> Result<db::models::Command, Error> {
        use db::schema::commands::dsl;
        let c = self.0.pool.lock();

        let filter =
            dsl::commands.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));

        match filter
            .clone()
            .first::<db::models::Command>(&*c)
            .optional()?
        {
            None => {
                let command = db::models::Command {
                    channel: key.channel.to_string(),
                    pattern: None,
                    name: key.name.to_string(),
                    count: 0,
                    text: text.to_string(),
                    group: None,
                    disabled: false,
                };

                diesel::insert_into(dsl::commands)
                    .values(&command)
                    .execute(&*c)?;
                return Ok(command);
            }
            Some(command) => {
                let mut set = db::models::UpdateCommand::default();
                set.text = Some(text);
                diesel::update(filter).set(&set).execute(&*c)?;
                return Ok(command);
            }
        }
    }

    /// Edit the pattern of a command.
    fn edit_pattern(
        &self,
        key: &db::Key,
        pattern: Option<&regex::Regex>,
    ) -> Result<(), failure::Error> {
        use db::schema::commands::dsl;
        let c = self.0.pool.lock();

        let pattern = pattern.map(|p| p.as_str());

        diesel::update(
            dsl::commands.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
        )
        .set(dsl::pattern.eq(pattern))
        .execute(&*c)?;

        Ok(())
    }

    /// Increment the given key.
    fn increment(&self, key: &db::Key) -> Result<bool, Error> {
        use db::schema::commands::dsl;

        let c = self.0.pool.lock();
        let count = diesel::update(
            dsl::commands.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
        )
        .set(dsl::count.eq(dsl::count + 1))
        .execute(&*c)?;
        Ok(count == 1)
    }
}

#[derive(Clone)]
pub struct Commands {
    inner: Arc<RwLock<db::Matcher<Command>>>,
    db: Database,
}

impl Commands {
    database_group_fns!(Command, db::Key);

    /// Construct a new commands store with a db.
    pub fn load(db: db::Database) -> Result<Commands, Error> {
        let db = Database(db);

        let mut matcher = db::Matcher::new();

        for command in db.list()? {
            let command = Command::from_db(&command)?;
            matcher.insert(command.key.clone(), Arc::new(command));
        }

        Ok(Commands {
            inner: Arc::new(RwLock::new(matcher)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub fn edit(
        &self,
        channel: &str,
        name: &str,
        template: template::Template,
    ) -> Result<(), Error> {
        let key = db::Key::new(channel, name);

        let command = self.db.edit(&key, template.source())?;

        if command.disabled {
            self.inner.write().remove(&key);
        } else {
            let vars = template.vars();

            let command = Arc::new(Command {
                key: key.clone(),
                pattern: db::Pattern::from_db(command.pattern.as_ref())?,
                count: Arc::new(AtomicUsize::new(command.count as usize)),
                template,
                vars,
                group: command.group,
                disabled: command.disabled,
            });

            self.inner.write().insert(key.clone(), command.clone());
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
            .modify_with_pattern(key, pattern, |value, pattern| {
                value.pattern = pattern;
            });

        Ok(())
    }

    /// Increment the specified command.
    pub fn increment(&self, command: &Command) -> Result<(), Error> {
        self.db.increment(&command.key)?;
        command.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Resolve the given command.
    pub fn resolve<'a>(
        &self,
        channel: &str,
        first: Option<&'a str>,
        it: &utils::Words<'a>,
    ) -> Option<(Arc<Command>, db::Captures<'a>)> {
        self.inner
            .read()
            .resolve(channel, first, it)
            .map(|(command, captures)| (command.clone(), captures))
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Command {
    /// Key for the command.
    pub key: db::Key,
    /// Pattern to use for matching command.
    pub pattern: db::Pattern,
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
    S: serde::Serializer,
{
    use serde::Serialize as _;

    value
        .load(std::sync::atomic::Ordering::SeqCst)
        .serialize(serializer)
}

impl Command {
    pub const NAME: &'static str = "command";

    /// Load a command from the database.
    pub fn from_db(command: &db::models::Command) -> Result<Command, Error> {
        let template = template::Template::compile(&command.text)
            .with_context(|_| format_err!("failed to compile command `{:?}` from db", command))?;

        let key = db::Key::new(&command.channel, &command.name);
        let count = Arc::new(AtomicUsize::new(command.count as usize));
        let vars = template.vars();

        let pattern = db::Pattern::from_db(command.pattern.as_ref())?;

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

    /// Get the currenct count.
    pub fn count(&self) -> i32 {
        self.count.load(Ordering::SeqCst) as i32
    }

    /// Render the given command.
    pub fn render<T>(&self, data: &T) -> Result<String, Error>
    where
        T: serde::Serialize,
    {
        Ok(self.template.render_to_string(data)?)
    }

    /// Test if the rendered command has the given var.
    pub fn has_var(&self, var: &str) -> bool {
        self.vars.contains(var)
    }
}

impl db::Matchable for Command {
    fn key(&self) -> &db::Key {
        &self.key
    }

    fn pattern(&self) -> &db::Pattern {
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
            group = self.group.as_ref().map(|g| g.as_str()).unwrap_or("*none*"),
            disabled = self.disabled,
        )
    }
}
