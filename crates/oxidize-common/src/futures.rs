use std::future::Future;
use std::pin::Pin;

use crate::stream::StreamExt;
use crate::BoxFuture;

/// Collection of boxed futures to drive.
pub type Futures<'a, O> = ::futures_util::stream::FuturesUnordered<BoxFuture<'a, O>>;

/// Run a collection of borrowed futures.
pub struct BorrowedFutures<'a, O> {
    inner: ::futures_util::stream::FuturesUnordered<Pin<&'a mut dyn Future<Output = O>>>,
}

impl<'a, O> BorrowedFutures<'a, O> {
    /// Push a borrowed future into the local collection.
    pub fn push(&mut self, future: Pin<&'a mut dyn Future<Output = O>>) {
        self.inner.push(future);
    }

    /// Wait for the next future to complete.
    pub async fn next(&mut self) -> Option<O> {
        self.inner.next().await
    }
}

impl<O> Default for BorrowedFutures<'_, O> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

#[macro_export]
macro_rules! local_join {
    ($out:ident => $($future:ident),* $(,)?) => {
        $(let $future = pin!($future);)*
        let mut $out = $crate::BorrowedFutures::<Result<()>>::default();
        $($out.push($future);)*
    }
}
