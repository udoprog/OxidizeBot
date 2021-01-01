use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

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

pub struct Handle<T> {
    handle: tokio::task::JoinHandle<T>,
}

impl<T> Future for Handle<T> {
    type Output = Result<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.handle).poll(cx) {
            Poll::Ready(output) => Poll::Ready(Ok(output?)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Spawn the given task in the background.
pub fn spawn<F>(future: F) -> Handle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let handle = tokio::spawn(future);
    Handle { handle }
}
