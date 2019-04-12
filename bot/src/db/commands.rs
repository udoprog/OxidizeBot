use crate::{db, template};
use diesel::prelude::*;
use failure::{format_err, ResultExt as _};
use hashbrown::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// The db of a words store.
trait Backend: Clone + Send + Sync {
    /// List all commands in db.
    fn list(&self) -> Result<Vec<db::models::Command>, failure::Error>;

    /// Insert or update an existing command.
    fn edit(&self, key: &Key, text: &str) -> Result<(), failure::Error>;

    /// Delete the given command from the db.
    fn delete(&self, key: &Key) -> Result<bool, failure::Error>;

    /// Increment the number of times the command has been invoked.
    /// Returns `true` if the counter existed and was incremented. `false` otherwise.
    fn increment(&self, key: &Key) -> Result<bool, failure::Error>;

    /// Rename the given command.
    fn rename(&self, from: &Key, to: &Key) -> Result<bool, failure::Error>;
}

impl Backend for db::Database {
    fn edit(&self, key: &Key, text: &str) -> Result<(), failure::Error> {
        use db::schema::commands::dsl;

        let c = self.pool.lock();
        let filter =
            dsl::commands.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)));
        let b = filter
            .clone()
            .first::<db::models::Command>(&*c)
            .optional()?;

        match b {
            None => {
                let command = db::models::Command {
                    channel: key.channel.to_string(),
                    name: key.name.to_string(),
                    count: 0,
                    text: text.to_string(),
                };

                diesel::insert_into(dsl::commands)
                    .values(&command)
                    .execute(&*c)?;
            }
            Some(_) => {
                diesel::update(filter)
                    .set(dsl::text.eq(text))
                    .execute(&*c)?;
            }
        }

        Ok(())
    }

    fn delete(&self, key: &Key) -> Result<bool, failure::Error> {
        use db::schema::commands::dsl;

        let c = self.pool.lock();
        let count = diesel::delete(
            dsl::commands.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
        )
        .execute(&*c)?;
        Ok(count == 1)
    }

    fn list(&self) -> Result<Vec<db::models::Command>, failure::Error> {
        use db::schema::commands::dsl;
        let c = self.pool.lock();
        Ok(dsl::commands.load::<db::models::Command>(&*c)?)
    }

    fn increment(&self, key: &Key) -> Result<bool, failure::Error> {
        use db::schema::commands::dsl;

        let c = self.pool.lock();
        let count = diesel::update(
            dsl::commands.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
        )
        .set(dsl::count.eq(dsl::count + 1))
        .execute(&*c)?;
        Ok(count == 1)
    }

    fn rename(&self, from: &Key, to: &Key) -> Result<bool, failure::Error> {
        use db::schema::commands::dsl;

        let c = self.pool.lock();
        let count = diesel::update(
            dsl::commands.filter(dsl::channel.eq(&from.channel).and(dsl::name.eq(&from.name))),
        )
        .set((dsl::channel.eq(&to.channel), dsl::name.eq(&to.name)))
        .execute(&*c)?;

        Ok(count == 1)
    }
}

#[derive(Clone)]
pub struct Commands {
    inner: Arc<RwLock<HashMap<Key, Arc<Command>>>>,
    db: db::Database,
}

impl Commands {
    /// Construct a new commands store with a db.
    pub fn load(db: db::Database) -> Result<Commands, failure::Error> {
        let mut inner = HashMap::new();

        for command in db.list()? {
            let template = template::Template::compile(&command.text).with_context(|_| {
                format_err!("failed to compile command `{:?}` from db", command)
            })?;

            let key = Key::new(command.channel.as_str(), command.name.as_str());
            let count = Arc::new(AtomicUsize::new(command.count as usize));
            let vars = template.vars();

            inner.insert(
                key.clone(),
                Arc::new(Command {
                    key,
                    count,
                    template,
                    vars,
                }),
            );
        }

        Ok(Commands {
            inner: Arc::new(RwLock::new(inner)),
            db,
        })
    }

    /// Insert a word into the bad words list.
    pub fn edit(&self, channel: &str, name: &str, command: &str) -> Result<(), failure::Error> {
        let key = Key::new(channel, name);

        let template = template::Template::compile(command)?;
        self.db.edit(&key, command)?;

        let mut inner = self.inner.write();
        let count = inner.get(&key).map(|c| c.count()).unwrap_or(0);

        let vars = template.vars();

        inner.insert(
            key.clone(),
            Arc::new(Command {
                key,
                // use integer atomics when available.
                count: Arc::new(AtomicUsize::new(count as usize)),
                template,
                vars,
            }),
        );

        Ok(())
    }

    /// Remove command.
    pub fn delete(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
        let key = Key::new(channel, name);

        if !self.db.delete(&key)? {
            return Ok(false);
        }

        self.inner.write().remove(&key);
        Ok(true)
    }

    /// Test the given word.
    pub fn get<'a>(&'a self, channel: &str, name: &str) -> Option<Arc<Command>> {
        let key = Key::new(channel, name);

        let inner = self.inner.read();

        if let Some(command) = inner.get(&key) {
            return Some(Arc::clone(command));
        }

        None
    }

    /// Get a list of all commands.
    pub fn list(&self, channel: &str) -> Vec<Arc<Command>> {
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

    /// Increment the specified command.
    pub fn increment(&self, command: &Command) -> Result<(), failure::Error> {
        self.db.increment(&command.key)?;
        command.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Try to rename the command.
    pub fn rename(&self, channel: &str, from: &str, to: &str) -> Result<(), super::RenameError> {
        let from = Key::new(channel, from);
        let to = Key::new(channel, to);

        let mut inner = self.inner.write();

        if inner.contains_key(&to) {
            return Err(super::RenameError::Conflict);
        }

        let command = match inner.remove(&from) {
            Some(command) => command,
            None => return Err(super::RenameError::Missing),
        };

        let command = Command {
            key: to.clone(),
            count: command.count.clone(),
            template: command.template.clone(),
            vars: command.vars.clone(),
        };

        match self.db.rename(&from, &to) {
            Err(e) => {
                log::error!(
                    "failed to rename command `{}` in database: {}",
                    from.name,
                    e
                );
            }
            Ok(false) => {
                log::warn!("command {} not renamed in database", from.name);
            }
            Ok(true) => (),
        }

        inner.insert(to, Arc::new(command));
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug)]
pub struct Command {
    pub key: Key,
    count: Arc<AtomicUsize>,
    pub template: template::Template,
    vars: HashSet<String>,
}

impl Command {
    /// Get the currenct count.
    pub fn count(&self) -> i32 {
        self.count.load(Ordering::SeqCst) as i32
    }

    /// Render the given command.
    pub fn render<T>(&self, data: &T) -> Result<String, failure::Error>
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
