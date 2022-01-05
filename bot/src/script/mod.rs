use crate::command;
use crate::db;
use anyhow::{anyhow, Result};
use ignore::Walk;
use rune::runtime::{ConstValue, Protocol, RuntimeContext, SyncFunction};
use rune::termcolor;
use rune::{
    Any, Context, ContextError, Diagnostics, FromValue, Module, Options, Source, Sources, Vm,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod io;

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

            if path.extension() != Some(OsStr::new("rn")) {
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

struct InternalHandler {
    name: String,
    function: SyncFunction,
    path: PathBuf,
    sources: Arc<Sources>,
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
#[derive(Clone, Any)]
struct ScopedDb {
    command: String,
    db: db::ScriptStorage,
}

impl ScopedDb {
    /// Set the given value in the database.
    async fn set(&self, key: ConstValue, value: ConstValue) -> Result<(), rune::Error> {
        self.db.set(key, value).await?;
        Ok(())
    }

    /// Get the stored value.
    async fn get(&self, key: ConstValue) -> Result<Option<ConstValue>, rune::Error> {
        let value = self.db.get::<ConstValue, ConstValue>(key).await?;
        Ok(value)
    }
}

pub(crate) struct Handler {
    db: Db,
    handler: Arc<InternalHandler>,
}

impl Handler {
    /// Call the given handler with the current context.
    pub(crate) async fn call(self, ctx: command::Context) -> Result<()> {
        let ctx = Ctx {
            ctx,
            db: self.db.scoped(&self.handler.name),
        };

        let result: Result<(), ConstValue> =
            match self.handler.function.async_send_call((ctx,)).await {
                Ok(result) => result,
                Err(error) => {
                    let mut buffer = termcolor::Buffer::no_color();
                    error.emit(&mut buffer, &*self.handler.sources)?;

                    return Err(anyhow!(
                        "failed to call handler for: {}:\n{}",
                        self.handler.name,
                        String::from_utf8(buffer.into_inner())?
                    ));
                }
            };

        match result {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow!(
                "error when calling handler: {}: {:?}",
                self.handler.name,
                e
            )),
        }
    }
}

pub(crate) struct Scripts {
    context: Arc<Context>,
    runtime: Arc<RuntimeContext>,
    options: Options,
    db: Db,
    handlers: HashMap<String, Arc<InternalHandler>>,
    // Keeps track of commands by path so that they may be unregistered.
    handlers_by_path: HashMap<PathBuf, Vec<String>>,
}

impl Scripts {
    /// Construct a new script handler.
    async fn new(channel: String, db: db::Database) -> Result<Self> {
        let context = Self::context()?;
        let runtime = Arc::new(context.runtime());

        Ok(Self {
            context,
            runtime,
            options: Default::default(),
            db: Db::open(channel, db).await?,
            handlers: HashMap::new(),
            handlers_by_path: HashMap::new(),
        })
    }

    /// Get the handler for the given name.
    pub(crate) fn get(&self, name: &str) -> Option<Handler> {
        let handler = self.handlers.get(name)?.clone();

        Some(Handler {
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
        let mut sources = Sources::new();
        sources.insert(Source::from_path(path)?);

        let mut diagnostics = Diagnostics::new();

        let result = rune::prepare(&mut sources)
            .with_context(&self.context)
            .with_options(&self.options)
            .with_diagnostics(&mut diagnostics)
            .build();

        let sources = Arc::new(sources);

        let unit = match result {
            Ok(unit) => Arc::new(unit),
            Err(..) => {
                let mut buffer = termcolor::Buffer::no_color();
                diagnostics.emit(&mut buffer, &*sources)?;

                return Err(anyhow!(
                    "failed to load source at: {}:\n{}",
                    path.display(),
                    String::from_utf8(buffer.into_inner())?
                ));
            }
        };

        let mut reg = Registry::new();
        let mut vm = Vm::new(self.runtime.clone(), unit);
        <()>::from_value(vm.call(&["main"], (&mut reg,))?)?;

        for (command, function) in reg.handlers {
            if let Some(handler) = self.handlers.get(&command) {
                log::warn!(
                    "ignoring duplicate handler for command `{}`, already registered in {}",
                    command,
                    handler.path.display()
                );
                continue;
            }

            let handler = Arc::new(InternalHandler {
                name: command.clone(),
                function,
                path: path.to_owned(),
                sources: sources.clone(),
            });

            self.handlers.insert(command.clone(), handler);
            self.handlers_by_path
                .entry(path.to_owned())
                .or_default()
                .push(command);
        }

        Ok(())
    }

    /// Construct a new context.
    fn context() -> Result<Arc<Context>, ContextError> {
        let mut ctx = rune_modules::with_config(false)?;
        ctx.install(&Self::oxi_mod()?)?;
        ctx.install(&self::io::module()?)?;
        Ok(Arc::new(ctx))
    }

    fn oxi_mod() -> Result<Module, ContextError> {
        let mut m = Module::with_item(&["oxi"]);

        m.ty::<Ctx>()?;
        m.async_inst_fn("respond", Ctx::respond)?;
        m.async_inst_fn("privmsg", Ctx::privmsg)?;
        m.inst_fn("user", Ctx::user)?;
        m.field_fn(Protocol::GET, "db", Ctx::db)?;

        m.ty::<Registry>()?;
        m.inst_fn("register", Registry::register)?;

        m.ty::<ScopedDb>()?;
        m.async_inst_fn(Protocol::INDEX_SET, ScopedDb::set)?;
        m.async_inst_fn("set", ScopedDb::set)?;
        m.async_inst_fn(Protocol::INDEX_GET, ScopedDb::get)?;
        m.async_inst_fn("get", ScopedDb::get)?;

        Ok(m)
    }
}

#[derive(Any)]
struct Registry {
    handlers: HashMap<String, SyncFunction>,
}

impl Registry {
    /// Construct a new registry.
    fn new() -> Self {
        Self {
            handlers: Default::default(),
        }
    }

    /// Register the given handler.
    fn register(&mut self, name: &str, handler: SyncFunction) {
        self.handlers.insert(name.to_owned(), handler);
    }
}

#[derive(Clone, Any)]
struct Ctx {
    ctx: command::Context,
    db: ScopedDb,
}

impl Ctx {
    /// Access the db associated with the context.
    fn db(&self) -> ScopedDb {
        self.db.clone()
    }

    /// Get the user name, if present.
    fn user(&self) -> Option<String> {
        self.ctx.user.name().map(|s| s.to_owned())
    }

    /// Respond with the given message.
    async fn respond(&self, message: &str) {
        self.ctx.respond(message).await;
    }

    /// Send a privmsg, without prefixing it with the user we are responding to.
    async fn privmsg(&self, message: &str) {
        self.ctx.privmsg(message).await;
    }
}
