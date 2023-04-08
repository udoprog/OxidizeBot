use crate::command;
use crate::db;
use anyhow::Result;
use std::path::Path;

pub(crate) async fn load_dir<I>(_channel: String, _db: db::Database, _paths: I) -> Result<Scripts>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    Ok(Scripts(()))
}

pub(crate) struct Handler(());

impl Handler {
    pub(crate) async fn call(self, _ctx: command::Context) -> Result<()> {
        Ok(())
    }
}

pub(crate) struct Scripts(());

impl Scripts {
    pub(crate) fn get(&self, _name: &str) -> Option<Handler> {
        None
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn reload(&mut self, path: &Path) -> Result<()> {
        tracing::trace!("Reload");
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn unload(&mut self, path: &Path) {
        tracing::trace!("Unload");
    }
}
