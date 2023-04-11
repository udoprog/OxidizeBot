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

pub use futures_util::sink;
pub use tokio_stream as stream;

pub mod models;

pub mod backoff;

mod uri;
pub use self::uri::Uri;

mod percentage;
pub use self::percentage::percentage;

#[macro_use]
mod futures;
pub use self::futures::{BorrowedFutures, Futures, LocalFutures};

/// A boxed future.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// A boxed stream.
pub type BoxStream<'a, T> = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = T> + Send + 'a>>;

/// This is a function which `Pin<Box<T>>`'s something if we're running a debug
/// build. Otherwise the value is just passed straight through.
///
/// This is used for really large futures which tend to blow up the stack in
/// debug mode.
#[cfg(debug_assertions)]
#[inline(always)]
pub fn debug_box_pin<T>(input: T) -> std::pin::Pin<Box<T>> {
    Box::pin(input)
}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn debug_box_pin<T>(input: T) -> T {
    input
}
