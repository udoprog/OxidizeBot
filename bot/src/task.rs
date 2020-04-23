use std::future::Future;

/// Asyncify the given task.
pub async fn asyncify<F, T, E>(task: F) -> Result<T, E>
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

/// Spawn the given task in the background.
pub fn spawn<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let _ = tokio::spawn(future);
}
