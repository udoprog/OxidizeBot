use futures::{compat::Compat01As03, future::FusedFuture, stream::FusedStream, Future, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub struct Interval {
    inner: Compat01As03<tokio::timer::Interval>,
}

impl Interval {
    pin_utils::unsafe_pinned!(inner: Compat01As03<tokio::timer::Interval>);
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
            inner: Compat01As03::new(tokio::timer::Interval::new_interval(duration)),
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
            inner: Compat01As03::new(tokio::timer::Interval::new(at, duration)),
        }
    }
}

impl Stream for Interval {
    type Item = Result<Instant, failure::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.inner().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => match result {
                Some(result) => match result {
                    Ok(instant) => Poll::Ready(Some(Ok(instant))),
                    Err(e) => Poll::Ready(Some(Err(e.into()))),
                },
                None => Poll::Ready(None),
            },
        }
    }
}

impl FusedStream for Interval {
    fn is_terminated(&self) -> bool {
        false
    }
}

pub struct Delay {
    terminated: bool,
    inner: Compat01As03<tokio::timer::Delay>,
}

impl Delay {
    /// Create a new `Delay` instance that elapses at `deadline`.
    ///
    /// Only millisecond level resolution is guaranteed. There is no guarantee
    /// as to how the sub-millisecond portion of `deadline` will be handled.
    /// `Delay` should not be used for high-resolution timer use cases.
    pub fn new(deadline: Instant) -> Self {
        Self {
            terminated: false,
            inner: Compat01As03::new(tokio::timer::Delay::new(deadline)),
        }
    }
}

impl Future for Delay {
    type Output = Result<(), failure::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let poll = match Pin::new(&mut self.inner).poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(result) => match result {
                Ok(()) => Poll::Ready(Ok(())),
                Err(e) => Poll::Ready(Err(e.into())),
            },
        };

        self.as_mut().terminated = true;
        poll
    }
}

impl FusedFuture for Delay {
    fn is_terminated(&self) -> bool {
        self.terminated
    }
}
