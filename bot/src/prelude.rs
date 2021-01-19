pub use crate::injector;
pub use crate::settings;
pub(crate) use crate::utils;
pub use async_fuse::Fuse;
pub use async_trait::async_trait;
pub use futures_core::Stream;
pub use std::future::Future;
pub use std::pin::Pin;
pub use std::sync::Arc;
pub use std::task::{Context, Poll};
pub use tokio::sync::{mpsc, oneshot};

/// A boxed future.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A boxed stream.
pub type BoxStream<'a, T> = Pin<Box<dyn futures_core::Stream<Item = T> + Send + 'a>>;

pub use futures_util::{StreamExt as _, TryStreamExt as _};
