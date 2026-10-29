use std::collections::HashMap;
use std::ffi::OsStr;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use common::Channel;
use ignore::Walk;
use rune::runtime::{Function, Protocol, Ref, RuntimeContext, SyncFunction, VmResult};
use rune::termcolor;
use rune::{Any, Context, ContextError, Diagnostics, Module, Options, Source, Sources, Vm};

use crate::chat;
use crate::command;
use crate::script::io;
use crate::script::value::StoredValue;

/// Load all scripts from the given directory.
pub(crate) async fn load_dir<I>(channel: &Channel, db: db::Database, paths: I) -> Result<Scripts>
where
    I: IntoIterator<Item: AsRef<Path>>,
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
                common::log_error!(e, "Failed to load script: {}", path.display())
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
    fn new(channel: &Channel, db: db::Database) -> Self {
        Self {
            db: db::ScriptStorage::new(channel, db),
        }
    }

    /// Scope the db.
    fn scoped(&self) -> ScopedDb {
        ScopedDb {
            db: self.db.clone(),
        }
    }
}

/// A database scoped into a single command.
#[derive(Clone, Any)]
struct ScopedDb {
    db: db::ScriptStorage,
}

impl ScopedDb {
    /// Set the given value in the database.
    fn set(
        this: Ref<Self>,
        key: StoredValue,
        value: StoredValue,
    ) -> impl Future<Output = Result<(), String>> {
        let db = this.db.clone();

        async move {
            db.set(key, value).await.map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    /// Get the stored value.
    fn get(
        this: Ref<Self>,
        key: StoredValue,
    ) -> impl Future<Output = Result<Option<StoredValue>, String>> {
        let db = this.db.clone();

        async move {
            db.get::<StoredValue, StoredValue>(key)
                .await
                .map_err(|e| e.to_string())
        }
    }
}

pub(crate) struct Handler {
    db: Db,
    handler: Arc<InternalHandler>,
}

impl Handler {
    /// Call the given handler with the current context.
    pub(crate) async fn call(self, ctx: command::Context<'_>) -> Result<()> {
        let ctx = Ctx {
            user: ctx.user,
            db: self.db.scoped(),
        };

        let result: Result<(), StoredValue> = match self
            .handler
            .function
            .async_send_call((ctx,))
            .await
            .into_result()
        {
            Ok(result) => result,
            Err(error) => {
                let mut buffer = termcolor::Buffer::no_color();
                error.emit(&mut buffer, &self.handler.sources)?;

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
    async fn new(channel: &Channel, db: db::Database) -> Result<Self> {
        let context = Self::context()?;
        let runtime = Arc::new(context.runtime()?);

        Ok(Self {
            context,
            runtime,
            options: Default::default(),
            db: Db::new(channel, db),
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
        sources.insert(Source::from_path(path)?)?;

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
                diagnostics.emit(&mut buffer, &sources)?;

                return Err(anyhow!(
                    "failed to load source at: {}:\n{}",
                    path.display(),
                    String::from_utf8(buffer.into_inner())?
                ));
            }
        };

        let mut reg = Registry::new();
        let mut vm = Vm::new(self.runtime.clone(), unit);
        vm.call(["main"], (&mut reg,))?.into_unit()?;

        for (command, function) in reg.handlers {
            if let Some(handler) = self.handlers.get(&command) {
                tracing::warn!(
                    "Ignoring duplicate handler for command `{}`, already registered in {}",
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
        ctx.install(Self::oxi_mod()?)?;
        ctx.install(self::io::module()?)?;
        Ok(Arc::new(ctx))
    }

    fn oxi_mod() -> Result<Module, ContextError> {
        let mut m = Module::with_item(["oxi"])?;

        m.ty::<Ctx>()?;
        m.associated_function("respond", Ctx::respond)?;
        m.associated_function("privmsg", Ctx::privmsg)?;
        m.associated_function("user", Ctx::user)?;
        m.field_function(&Protocol::GET, "db", Ctx::db)?;

        m.ty::<Registry>()?;
        m.associated_function("register", Registry::register)?;

        m.ty::<ScopedDb>()?;
        m.associated_function(&Protocol::INDEX_SET, ScopedDb::set)?;
        m.associated_function("set", ScopedDb::set)?;
        m.associated_function(&Protocol::INDEX_GET, ScopedDb::get)?;
        m.associated_function("get", ScopedDb::get)?;

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
    fn register(&mut self, name: &str, handler: Function) -> VmResult<()> {
        let handler = rune::vm_try!(handler.into_sync());
        self.handlers.insert(name.to_owned(), handler);
        VmResult::Ok(())
    }
}

#[derive(Clone, Any)]
struct Ctx {
    user: chat::User,
    db: ScopedDb,
}

impl Ctx {
    /// Access the db associated with the context.
    fn db(&self) -> ScopedDb {
        self.db.clone()
    }

    /// Get the user name, if present.
    fn user(&self) -> Option<String> {
        self.user.name().map(|s| s.to_owned())
    }

    /// Respond with the given message.
    fn respond(this: Ref<Self>, message: Ref<str>) -> impl Future<Output = ()> {
        let user = this.user.clone();
        async move { user.respond(&*message).await }
    }

    /// Send a privmsg, without prefixing it with the user we are responding to.
    fn privmsg(this: Ref<Self>, message: Ref<str>) -> impl Future<Output = ()> {
        let user = this.user.clone();
        async move { user.sender().privmsg(&*message).await }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use rune::{Diagnostics, Source, Sources, Vm};

    use super::{Registry, Scripts};

    #[test]
    fn example_script_registers() -> anyhow::Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/dynamic.rn");

        let context = Scripts::context()?;
        let mut sources = Sources::new();
        sources.insert(Source::from_path(&path)?)?;
        let mut diagnostics = Diagnostics::new();

        let unit = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build();

        assert!(
            diagnostics.is_empty(),
            "diagnostics: {:?}",
            diagnostics.diagnostics()
        );

        let mut reg = Registry::new();
        let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit?));
        vm.call(["main"], (&mut reg,))?.into_unit()?;
        assert!(reg.handlers.contains_key("dynamic"));
        Ok(())
    }
}
