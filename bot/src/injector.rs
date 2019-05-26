use futures::{channel::mpsc, ready, stream};
use hashbrown::HashMap;
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

        match (value as Box<Any + 'static>).downcast::<T>() {
            Ok(value) => Poll::Ready(Some(Some(*value))),
            Err(_) => panic!("downcast failed"),
        }
    }
}

#[derive(Default)]
struct Inner {
    storage: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>,
    subs: HashMap<TypeId, Vec<Sender>>,
}

/// Use for handling injection.
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
        T: Any + Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();

        let mut inner = self.inner.write();

        if let None = inner.storage.remove(&id) {
            return;
        }

        if let Some(subs) = inner.subs.get(&id) {
            for s in subs {
                if let Err(e) = s.tx.unbounded_send(None) {
                    log::warn!("failed to send resource update: {}", e);
                }
            }
        }
    }

    /// Set the given value and notify any subscribers.
    pub fn update<T>(&self, value: T)
    where
        T: Any + Send + Sync + 'static + Clone,
    {
        let id = TypeId::of::<T>();
        let value = Box::new(value);
        let mut inner = self.inner.write();

        if let Some(subs) = inner.subs.get(&id) {
            for s in subs {
                if let Err(e) = s.tx.unbounded_send(Some(value.clone())) {
                    log::warn!("failed to send resource update: {}", e);
                }
            }
        }

        inner.storage.insert(id, value);
    }

    /// Get an existing value and setup a stream for updates at the same time.
    pub fn stream<T>(&self) -> (Stream<T>, Option<T>)
    where
        T: Any + Send + Sync + 'static + Clone,
    {
        let id = TypeId::of::<T>();

        let mut inner = self.inner.write();

        let value = match inner.storage.get(&id) {
            Some(value) => match value.downcast_ref::<T>() {
                Some(value) => Some(value.clone()),
                None => panic!("downcast failed"),
            },
            None => None,
        };

        let (tx, rx) = mpsc::unbounded();
        inner.subs.entry(id).or_default().push(Sender { tx });

        let stream = Stream {
            rx,
            marker: marker::PhantomData,
        };

        (stream, value)
    }
}
