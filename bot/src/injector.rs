use crate::utils;
use futures::{channel::mpsc, ready, stream};
use hashbrown::{HashMap, HashSet};
use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    marker,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// Use for sending information on updates.
struct Sender {
    /// Unique id for this sender.
    id: u32,
    tx: mpsc::UnboundedSender<Option<Box<dyn Any + Send + Sync + 'static>>>,
}

/// A stream of updates for values injected into this injector.
pub struct Stream<T> {
    rx: mpsc::UnboundedReceiver<Option<Box<dyn Any + Send + Sync + 'static>>>,
    marker: marker::PhantomData<T>,
}

impl<T> stream::Stream for Stream<T>
where
    T: Unpin + Any + Send + Sync + 'static,
{
    type Item = Option<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let value = match ready!(Pin::new(&mut self.rx).poll_next(cx)) {
            Some(Some(value)) => value,
            Some(None) => return Poll::Ready(Some(None)),
            None => return Poll::Ready(None),
        };

        match (value as Box<dyn Any + 'static>).downcast::<T>() {
            Ok(value) => Poll::Ready(Some(Some(*value))),
            Err(_) => panic!("downcast failed"),
        }
    }
}

impl<T> stream::FusedStream for Stream<T> {
    fn is_terminated(&self) -> bool {
        false
    }
}

#[derive(Default)]
struct Inner {
    id: u32,
    storage: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>,
    subs: HashMap<TypeId, Vec<Sender>>,
}

/// Use for handling injection.
#[derive(Clone)]
pub struct Injector {
    inner: Arc<RwLock<Inner>>,
}

impl Injector {
    /// Create a new injector instance.
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    /// Clear the given value.
    pub fn clear<T>(&self)
    where
        T: Clone + Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        let mut inner = self.inner.write();

        if let None = inner.storage.remove(&type_id) {
            return;
        }

        self.try_send(&mut *inner, type_id, || None);
    }

    /// Set the given value and notify any subscribers.
    pub fn update<T>(&self, value: T)
    where
        T: Any + Send + Sync + 'static + Clone,
    {
        let type_id = TypeId::of::<T>();
        let mut inner = self.inner.write();
        self.try_send(&mut *inner, type_id, || Some(Box::new(value.clone())));
        inner.storage.insert(type_id, Box::new(value));
    }

    /// Get a value from the injector.
    pub fn get<T>(&self) -> Option<T>
    where
        T: Any + Send + Sync + 'static + Clone,
    {
        let type_id = TypeId::of::<T>();
        let inner = self.inner.read();

        match inner.storage.get(&type_id) {
            Some(value) => match value.downcast_ref::<T>() {
                Some(value) => Some(value.clone()),
                None => panic!("downcast failed"),
            },
            None => None,
        }
    }

    /// Get an existing value and setup a stream for updates at the same time.
    pub fn stream<T>(&self) -> (Stream<T>, Option<T>)
    where
        T: Any + Send + Sync + 'static + Clone,
    {
        let type_id = TypeId::of::<T>();

        let mut inner = self.inner.write();

        let value = match inner.storage.get(&type_id) {
            Some(value) => match value.downcast_ref::<T>() {
                Some(value) => Some(value.clone()),
                None => panic!("downcast failed"),
            },
            None => None,
        };

        let id = inner.id;
        inner.id += 1;
        let (tx, rx) = mpsc::unbounded();
        inner
            .subs
            .entry(type_id)
            .or_default()
            .push(Sender { id, tx });

        let stream = Stream {
            rx,
            marker: marker::PhantomData,
        };

        (stream, value)
    }

    /// Get a synchronized variable for the given configuration key.
    pub fn var<'a, T, D>(&self, driver: &mut D) -> Arc<RwLock<Option<T>>>
    where
        T: Any + Send + Sync + 'static + Clone + Unpin,
        D: utils::Driver<'a>,
    {
        use futures::StreamExt as _;

        let (mut stream, value) = self.stream();
        let value = Arc::new(RwLock::new(value));
        let future_value = value.clone();

        let future = async move {
            while let Some(update) = stream.next().await {
                *future_value.write() = update;
            }

            Ok(())
        };

        driver.drive(future);
        value
    }

    /// Try to perform a send, or clean up if one fails.
    fn try_send<S>(&self, inner: &mut Inner, type_id: TypeId, send: S)
    where
        S: Fn() -> Option<Box<dyn Any + Send + Sync + 'static>>,
    {
        let mut to_delete = smallvec::SmallVec::<[u32; 16]>::new();

        if let Some(subs) = inner.subs.get(&type_id) {
            for s in subs {
                if let Err(e) = s.tx.unbounded_send(send()) {
                    if e.is_disconnected() {
                        to_delete.push(s.id);
                        continue;
                    }

                    log::warn!("failed to send resource update: {}", e);
                }
            }
        }

        if to_delete.is_empty() {
            return;
        }

        let to_delete = to_delete.into_iter().collect::<HashSet<_>>();

        if let Some(subs) = inner.subs.get_mut(&type_id) {
            let new_subs = subs
                .drain(..)
                .into_iter()
                .filter(|s| !to_delete.contains(&s.id))
                .collect();
            *subs = new_subs;
        }
    }
}
