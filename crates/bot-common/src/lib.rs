#[macro_use]
mod macros;

pub mod display;
pub mod irc;

mod cooldown;
pub use self::cooldown::Cooldown;

mod channel;
pub use self::channel::{Channel, OwnedChannel};

pub mod duration;
pub use self::duration::Duration;

pub mod words;

mod offset;
pub use self::offset::Offset;

pub mod tags;

mod pt_duration;
pub use self::pt_duration::PtDuration;

pub use tokio_stream as stream;
pub use futures_util::sink as sink;

pub mod models;

pub mod backoff;

mod uri;
pub use self::uri::Uri;

/// A boxed future.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// A boxed stream.
pub type BoxStream<'a, T> = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = T> + Send + 'a>>;

/// Collection of boxed futures to drive.
pub type Futures<'a, O> =
    ::futures_util::stream::FuturesUnordered<BoxFuture<'a, O>>;
