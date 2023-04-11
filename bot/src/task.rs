use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::Result;

pub struct Handle<O> {
    handle: tokio::task::JoinHandle<Result<O>>,
}

impl<O> Future for Handle<O> {
    type Output = Result<O>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.as_mut().handle).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Spawn the given task in the background.
pub(crate) fn spawn<F, O>(future: F) -> Handle<O>
where
    F: Future<Output = Result<O>> + Send + 'static,
    O: Send + 'static,
{
    Handle {
        handle: tokio::spawn(future),
    }
}
