use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Asyncify the given task.
pub(crate) async fn asyncify<F, T, E>(task: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
    E: From<tokio::task::JoinError>,
{
    match tokio::task::spawn_blocking(task).await {
        Ok(result) => result,
        Err(e) => Err(E::from(e)),
    }
}
