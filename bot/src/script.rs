use crate::command;
use crate::db;
use anyhow::{anyhow, Result};
use ignore::Walk;
use parking_lot::Mutex;
use rhai::de::from_dynamic;
use rhai::ser::to_dynamic;
use rhai::{
    Array, Dynamic, Engine, EvalAltResult, Module, RegisterFn as _, Scope, AST, FLOAT, INT,
};
use std::any::Any;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime;

/// Load all scripts from the given directory.
pub(crate) async fn load_dir<I>(channel: String, db: db::Database, paths: I) -> Result<Scripts>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    let mut scripts = Scripts::new(channel, db).await?;

    for path in paths {
        let path = path.as_ref();

        if !path.is_dir() {
            continue;
        }

        for result in Walk::new(path) {
            let entry = result?;
            let path = entry.path();

            if path.extension() != Some(OsStr::new("rhai")) {
                continue;
            }

            if !path.is_file() {
                continue;
            }

            let path = path.canonicalize()?;

            if let Err(e) = scripts.load(&path) {
                log_error!(e, "Failed to load script: {}", path.display())
            }
        }
    }

    Ok(scripts)
}

struct Handler {
    name: String,
    ast: Arc<AST>,
    path: PathBuf,
}

#[derive(Clone)]
struct Db {
    db: db::ScriptStorage,
}

impl Db {
    async fn open(channel: String, db: db::Database) -> Result<Self> {
        Ok(Self {
            db: db::ScriptStorage::load(channel, db).await?,
        })
    }

    /// Scope the db.
    fn scoped(&self, command: &str) -> ScopedDb {
        ScopedDb {
            command: command.to_owned(),
            db: self.db.clone(),
        }
    }
}

/// A database scoped into a single command.
#[derive(Clone)]
struct ScopedDb {
    command: String,
    db: db::ScriptStorage,
}

impl ScopedDb {
    /// Set the given value in the database.
    fn set<K, V>(&mut self, key: K, value: V) -> Result<(), Box<EvalAltResult>>
    where
        K: Clone + Send + Sync + Any,
        Dynamic: From<K>,
        V: Clone + Send + Sync + Any,
        Dynamic: From<V>,
    {
        let handle = runtime::Handle::current();

        let key = Dynamic::from(key);
        let value = Dynamic::from(value);

        let key: serde_cbor::Value = from_dynamic(&key)?;
        let value: serde_cbor::Value = from_dynamic(&value)?;

        handle.block_on(self.db.set(key, value)).map_err(|e| {
            Box::new(EvalAltResult::ErrorRuntime(
                format!("failed to update script key: {}", e),
                Default::default(),
            ))
        })?;

        Ok(())
    }

    /// Get the stored value.
    fn get<K>(&mut self, key: K) -> Result<Dynamic, Box<EvalAltResult>>
    where
        K: Clone + Send + Sync + Any,
        Dynamic: From<K>,
    {
        let key: serde_cbor::Value = from_dynamic(&Dynamic::from(key))?;

        let handle = runtime::Handle::current();

        let v: Option<serde_cbor::Value> = handle.block_on(self.db.get(key)).map_err(|e| {
            Box::new(EvalAltResult::ErrorRuntime(
                e.to_string(),
                Default::default(),
            ))
        })?;

        let v = match v {
            Some(v) => v,
            None => return Ok(Dynamic::from(())),
        };

        Ok(to_dynamic(v)?)
    }
}

pub(crate) struct EngineHandler {
    engine: Arc<Engine>,
    db: Db,
    handler: Arc<Handler>,
}

impl EngineHandler {
    /// Call the given handler with the current context.
    pub(crate) fn call(self, ctx: command::Context) -> Result<()> {
        let mut scope = Scope::new();
        scope.set_value("db", self.db.scoped(&self.handler.name));
        let ctx = Context { ctx: Arc::new(ctx) };

        let _: () = self
            .engine
            .call_fn(&mut scope, &*self.handler.ast, &self.handler.name, (ctx,))
            .map_err(|e| anyhow!("{}", e))?;

        Ok(())
    }
}

pub(crate) struct Scripts {
    engine: Arc<Engine>,
    db: Db,
    handlers: HashMap<String, Arc<Handler>>,
    // Keeps track of commands by path so that they may be unregistered.
    handlers_by_path: HashMap<PathBuf, Vec<String>>,
}

impl Scripts {
    /// Construct a new script handler.
    async fn new(channel: String, db: db::Database) -> Result<Self> {
        Ok(Self {
            engine: Self::new_engine(),
            db: Db::open(channel, db).await?,
            handlers: HashMap::new(),
            handlers_by_path: HashMap::new(),
        })
    }

    /// Get the handler for the given name.
    pub(crate) fn get(&self, name: &str) -> Option<EngineHandler> {
        let handler = self.handlers.get(name)?.clone();

        Some(EngineHandler {
            engine: self.engine.clone(),
            db: self.db.clone(),
            handler,
        })
    }

    /// Same as `load`, except that it removes the old handles before loading
    /// them again.
    pub(crate) fn reload(&mut self, path: &Path) -> Result<()> {
        self.unload(path);
        self.load(path)?;
        Ok(())
    }

    /// Unload all handlers associated with the given script path.
    pub(crate) fn unload(&mut self, path: &Path) {
        if let Some(old) = self.handlers_by_path.remove(path) {
            for command in old {
                self.handlers.remove(&command);
            }
        }
    }

    /// Load the given path as a script.
    pub(crate) fn load(&mut self, path: &Path) -> Result<()> {
        let source = fs::read_to_string(path)?;
        let ast = Arc::new(self.engine.compile(&source)?);
        let reg = Registry::new();

        let mut scope = Scope::new();

        let _: () = self
            .engine
            .call_fn(&mut scope, &*ast, "main", (reg.clone(),))
            .unwrap();

        let reg = Arc::try_unwrap(reg).map_err(|_| anyhow!("no unique access to registry"))?;

        for (command, name) in reg.into_handlers() {
            if let Some(handler) = self.handlers.get(&command) {
                log::warn!(
                    "ignoring duplicate handler for command `{}`, already registered in {}",
                    command,
                    handler.path.display()
                );
                continue;
            }

            let handler = Arc::new(Handler {
                name,
                ast: ast.clone(),
                path: path.to_owned(),
            });

            self.handlers.insert(command.clone(), handler);
            self.handlers_by_path
                .entry(path.to_owned())
                .or_default()
                .push(command);
        }

        Ok(())
    }

    /// Construct a new engine.
    fn new_engine() -> Arc<Engine> {
        let mut engine = Engine::new();
        engine.register_fn("register", Registry::register);
        engine.register_fn("respond", Context::respond);
        engine.register_fn("privmsg", Context::privmsg);
        engine.register_fn("user", Context::user);

        let mut db = Module::new();
        db.set_indexer_set_fn(ScopedDb::set::<Array, INT>);
        db.set_indexer_set_fn(ScopedDb::set::<Array, FLOAT>);
        db.set_indexer_get_fn(ScopedDb::get::<Array>);
        engine.load_package(Arc::new(db));

        Arc::new(engine)
    }
}

struct Registry {
    handlers: Mutex<HashMap<String, String>>,
}

impl Registry {
    /// Construct a new registry.
    fn new() -> Arc<Self> {
        Arc::new(Self {
            handlers: Default::default(),
        })
    }

    /// Register the given handler.
    fn register(self: Arc<Self>, name: &str, handler: &str) {
        self.handlers
            .lock()
            .insert(name.to_owned(), handler.to_owned());
    }

    /// Convert registry into handlers.
    fn into_handlers(self) -> HashMap<String, String> {
        self.handlers.into_inner()
    }
}

#[derive(Clone)]
struct Context {
    ctx: Arc<command::Context>,
}

impl Context {
    /// Get the user name, if present.
    fn user(self) -> Dynamic {
        self.ctx
            .user
            .name()
            .map(|s| Dynamic::from(s.to_owned()))
            .unwrap_or_default()
    }

    /// Respond with the given message.
    fn respond(self, message: &str) {
        let handle = runtime::Handle::current();
        handle.block_on(self.ctx.respond(message));
    }

    /// Send a privmsg, without prefixing it with the user we are responding to.
    fn privmsg(self, message: &str) {
        let handle = runtime::Handle::current();
        handle.block_on(self.ctx.privmsg(message));
    }
}
