use futures::{
    compat::Compat01As03, future, ready, stream, Future, FutureExt as _, Stream, StreamExt as _,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub struct Interval {
    inner: stream::Fuse<Compat01As03<tokio::timer::Interval>>,
}

impl Interval {
    /// Creates new `Interval` that yields with interval of `duration`.
    ///
    /// The function is shortcut for `Interval::new(Instant::now() + duration, duration)`.
    ///
    /// The `duration` argument must be a non-zero duration.
    ///
    /// # Panics
    ///
    /// This function panics if `duration` is zero.
    pub fn new_interval(duration: Duration) -> Self {
        Self {
            inner: Compat01As03::new(tokio::timer::Interval::new_interval(duration)).fuse(),
        }
    }

    /// Create a new `Interval` that starts at `at` and yields every `duration`
    /// interval after that.
    ///
    /// Note that when it starts, it produces item too.
    ///
    /// The `duration` argument must be a non-zero duration.
    ///
    /// # Panics
    ///
    /// This function panics if `duration` is zero.
    pub fn new(at: Instant, duration: Duration) -> Self {
        Self {
            inner: Compat01As03::new(tokio::timer::Interval::new(at, duration)).fuse(),
        }
    }
}

impl Stream for Interval {
    type Item = Result<Instant, failure::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Poll::Ready(match ready!(Pin::new(&mut self.inner).poll_next(cx)) {
            Some(result) => Some(Ok(result?)),
            None => None,
        })
    }
}

impl stream::FusedStream for Interval {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

pub struct Delay {
    inner: future::Fuse<Compat01As03<tokio::timer::Delay>>,
}

impl Delay {
    /// Create a new `Delay` instance that elapses at `deadline`.
    ///
    /// Only millisecond level resolution is guaranteed. There is no guarantee
    /// as to how the sub-millisecond portion of `deadline` will be handled.
    /// `Delay` should not be used for high-resolution timer use cases.
    pub fn new(deadline: Instant) -> Self {
        Self {
            inner: Compat01As03::new(tokio::timer::Delay::new(deadline)).fuse(),
        }
    }
}

impl Future for Delay {
    type Output = Result<(), failure::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map_err(Into::into)
    }
}

impl future::FusedFuture for Delay {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}
