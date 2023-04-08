use crate::sys::Notification;
use anyhow::Error;
use std::future;
use std::path::Path;

#[derive(Clone)]
pub(crate) struct System;

impl System {
    pub(crate) async fn wait_for_shutdown(&self) {
        future::pending().await
    }

    pub(crate) async fn wait_for_restart(&self) {
        future::pending().await
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
    Ok(System)
}
