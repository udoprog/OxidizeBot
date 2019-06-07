use failure::Error;
use futures::{channel::oneshot, future};
use std::path::Path;

#[derive(Clone)]
pub struct System;

impl System {
    pub async fn wait_for_shutdown(&self) -> Result<(), oneshot::Canceled> {
        future::empty().await
    }

    pub async fn wait_for_restart(&self) -> Result<(), oneshot::Canceled> {
        future::empty().await
    }

    pub fn is_running(&self) -> bool {
        true
    }

    pub fn join(&self) -> Result<(), Error> {
        Ok(())
    }
}

pub fn setup(_root: &Path, _log_file: &Path) -> Result<System, Error> {
    Ok(System)
}
