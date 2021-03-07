pub(crate) use crate::injector::{self, Injector, Key, Provider};
pub(crate) use crate::settings;
pub(crate) use crate::stream::{self as stream, StreamExt as _};
pub(crate) use crate::tags;
pub(crate) use crate::utils;
pub(crate) use async_fuse::Fuse;
pub(crate) use async_trait::async_trait;
pub(crate) use std::future::Future;
pub(crate) use std::pin::Pin;
pub(crate) use std::sync::Arc;
pub(crate) use std::task::{Context, Poll};
pub(crate) use tokio::sync::{mpsc, oneshot};

/// A boxed future.
pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A boxed stream.
pub(crate) type BoxStream<'a, T> = Pin<Box<dyn tokio_stream::Stream<Item = T> + Send + 'a>>;
