use std::path::Path;
use std::sync::Arc;

use anyhow::Error;
use tokio::sync::Notify;

use crate::sys::Notification;

#[derive(Clone)]
pub(crate) struct System {
    restart: Arc<Notify>,
}

impl System {
    pub(crate) async fn wait_for_shutdown(&self) {
        std::future::pending().await
    }

    pub(crate) async fn wait_for_restart(&self) {
        self.restart.notified().await;
    }

    pub(crate) fn restart(&self) -> &Arc<Notify> {
        &self.restart
    }

    pub(crate) fn clear(&self) {}

    pub(crate) fn error(&self, _error: String) {}

    pub(crate) fn notification(&self, _: Notification) {}

    pub(crate) fn join(&self) -> Result<(), Error> {
        Ok(())
    }

    pub(crate) fn is_installed(&self) -> Result<bool, Error> {
        Ok(true)
    }

    pub(crate) fn install(&self) -> Result<(), Error> {
        Ok(())
    }

    pub(crate) fn uninstall(&self) -> Result<(), Error> {
        Ok(())
    }
}

pub(crate) fn setup(_root: &Path, _log_file: &Path) -> Result<System, Error> {
    Ok(System {
        restart: Arc::new(Notify::default()),
    })
}
