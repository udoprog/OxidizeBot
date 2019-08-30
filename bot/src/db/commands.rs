use crate::{db, template};
use diesel::prelude::*;
use failure::{format_err, Error, ResultExt as _};
use hashbrown::{hash_map, HashMap, HashSet};
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
    private_database_group_fns!(commands, Command, Key);

    /// Edit the text for the given key.
    fn edit(&self, key: &Key, text: &str) -> Result<db::models::Command, Error> {
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
        key: &Key,
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
    fn increment(&self, key: &Key) -> Result<bool, Error> {
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

struct Inner {
    /// All commands.
    all: HashMap<Key, Arc<Command>>,
    /// Commands indexed by name.
    by_name: HashSet<Key>,
    /// Regular expression commands indexed by channel.
    by_channel_regex: HashMap<String, HashSet<Key>>,
}

impl Inner {
    /// Test if we contain the given key.
    pub fn contains_key(&self, key: &Key) -> bool {
        self.all.contains_key(key)
    }

    /// Insert the given command.
    pub fn insert(&mut self, key: Key, command: Arc<Command>) {
        match &command.pattern {
            Pattern::Name => {
                self.by_name.insert(key.clone());
            }
            Pattern::Regex { .. } => {
                self.by_channel_regex
                    .entry(key.channel.clone())
                    .or_default()
                    .insert(key.clone());
            }
        }

        self.all.insert(key, command);
    }

    /// Remove the given command.
    pub fn remove(&mut self, key: &Key) -> Option<Arc<Command>> {
        if let Some(command) = self.all.remove(key) {
            match &command.pattern {
                Pattern::Name => {
                    self.by_name.remove(key);
                }
                Pattern::Regex { .. } => {
                    self.by_channel_regex
                        .entry(key.channel.clone())
                        .or_default()
                        .remove(&key);
                }
            }

            return Some(command);
        }

        None
    }

    /// Get an iterator over all the values.
    pub fn iter(&self) -> hash_map::Iter<'_, Key, Arc<Command>> {
        self.all.iter()
    }

    /// Get an iterator over all the values.
    pub fn values(&self) -> hash_map::Values<'_, Key, Arc<Command>> {
        self.all.values()
    }

    /// Get the underlying key.
    pub fn get(&self, key: &Key) -> Option<&Arc<Command>> {
        self.all.get(key)
    }
}

#[derive(Clone)]
pub struct Commands {
    inner: Arc<RwLock<Inner>>,
    db: Database,
}

impl Commands {
    database_group_fns!(Command, Key);

    /// Construct a new commands store with a db.
    pub fn load(db: db::Database) -> Result<Commands, Error> {
        let db = Database(db);

        let mut all = HashMap::new();
        let mut by_name = HashSet::new();
        let mut by_channel_regex = HashMap::<String, HashSet<Key>>::new();

        for command in db.list()? {
            let command = Arc::new(Command::from_db(&command)?);

            match &command.pattern {
                Pattern::Name => {
                    by_name.insert(command.key.clone());
                }
                Pattern::Regex { .. } => {
                    by_channel_regex
                        .entry(command.key.channel.clone())
                        .or_default()
                        .insert(command.key.clone());
                }
            }

            all.insert(command.key.clone(), command.clone());
        }

        Ok(Commands {
            inner: Arc::new(RwLock::new(Inner {
                all,
                by_name,
                by_channel_regex,
            })),
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
        let key = Key::new(channel, name);

        let command = self.db.edit(&key, template.source())?;

        if command.disabled {
            self.inner.write().remove(&key);
        } else {
            let vars = template.vars();

            let command = Arc::new(Command {
                key: key.clone(),
                pattern: Pattern::from_db(command.pattern.as_ref())?,
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
        let key = Key::new(channel, name);
        self.db.edit_pattern(&key, pattern.as_ref())?;

        let Inner {
            all,
            by_channel_regex,
            by_name,
        } = &mut *self.inner.write();

        if let Some(existing) = all.get_mut(&key) {
            let pattern = if let Some(pattern) = pattern {
                if let Pattern::Name = existing.pattern {
                    by_name.remove(&key);

                    by_channel_regex
                        .entry(key.channel.clone())
                        .or_default()
                        .insert(key);
                }

                Pattern::Regex { pattern }
            } else {
                if let Pattern::Regex { .. } = existing.pattern {
                    by_channel_regex
                        .entry(key.channel.clone())
                        .or_default()
                        .remove(&key);

                    by_name.insert(key);
                } else {
                    // NB: nothing to do.
                    return Ok(());
                }

                Pattern::Name
            };

            let mut new = (**existing).clone();
            new.pattern = pattern;
            *existing = Arc::new(new);
        }

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
        first: Option<&str>,
        full: &'a str,
    ) -> Option<(Arc<Command>, Captures<'a>)> {
        let inner = self.inner.read();

        if let Some(first) = first {
            let key = Key::new(channel, first);

            if inner.by_name.contains(&key) {
                if let Some(command) = inner.get(&key) {
                    return Some((command.clone(), Default::default()));
                }
            }
        }

        if let Some(keys) = inner.by_channel_regex.get(channel) {
            for key in keys {
                if let Some(command) = inner.get(key) {
                    if let Pattern::Regex { pattern } = &command.pattern {
                        if let Some(captures) = pattern.captures(full) {
                            let captures = Captures {
                                captures: Some(captures),
                            };
                            return Some((command.clone(), captures));
                        }
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug, Default)]
pub struct Captures<'a> {
    captures: Option<regex::Captures<'a>>,
}

impl serde::Serialize for Captures<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap as _;

        let mut m = serializer.serialize_map(self.captures.as_ref().map(|c| c.len()))?;

        if let Some(captures) = &self.captures {
            for (i, g) in captures.iter().enumerate() {
                m.serialize_entry(&i, &g.map(|m| m.as_str()))?;
            }
        }

        m.end()
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

/// How to match the given command.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum Pattern {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "regex")]
    Regex {
        #[serde(serialize_with = "serialize_regex")]
        pattern: regex::Regex,
    },
}

impl Pattern {
    /// Convert a database pattern into a matchable pattern here.
    pub fn from_db(pattern: Option<impl AsRef<str>>) -> Result<Self, Error> {
        Ok(match pattern {
            Some(pattern) => Pattern::Regex {
                pattern: regex::Regex::new(pattern.as_ref())?,
            },
            None => Pattern::Name,
        })
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Name => "*name*".fmt(fmt),
            Pattern::Regex { pattern } => pattern.fmt(fmt),
        }
    }
}

/// Serialize a regular expression.
fn serialize_regex<S>(regex: &regex::Regex, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.collect_str(regex)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Command {
    /// Key for the command.
    pub key: Key,
    /// Pattern to use for matching command.
    pub pattern: Pattern,
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

        let key = Key::new(&command.channel, &command.name);
        let count = Arc::new(AtomicUsize::new(command.count as usize));
        let vars = template.vars();

        let pattern = Pattern::from_db(command.pattern.as_ref())?;

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
