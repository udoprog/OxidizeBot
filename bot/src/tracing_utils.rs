//! Tracing helpers for OxidizeBot.
//!
//! Adds tracing for entering and exiting spans.

use parking_lot::RwLock;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing_core::{
    span::{self, Id},
    Event, Metadata,
};

#[derive(Clone)]
struct SpanContext {
    file: Option<String>,
    line: Option<u32>,
    module_path: Option<String>,
    target: String,
    level: log::Level,
    name: &'static str,
}

impl SpanContext {
    /// Construct a new context.
    fn new(meta: &Metadata<'_>) -> Self {
        Self {
            file: meta.file().map(String::from),
            line: meta.line(),
            module_path: meta.module_path().map(String::from),
            target: String::from(meta.target()),
            level: tracing_to_log_level(meta.level()),
            name: meta.name(),
        }
    }

    fn log_meta(&self) -> log::Metadata<'_> {
        log::MetadataBuilder::new()
            .level(self.level)
            .target(self.target.as_ref())
            .build()
    }
}

/// A very simple logging subscriber that keeps track of the spans for each
/// thread and logs enter/exit points.
pub struct Subscriber {
    started_at: RwLock<HashMap<Id, Instant>>,
    spans: RwLock<HashMap<Id, Arc<SpanContext>>>,
    id_alloc: AtomicUsize,
}

impl Subscriber {
    pub fn new() -> Self {
        Self {
            started_at: Default::default(),
            spans: Default::default(),
            id_alloc: AtomicUsize::new(1),
        }
    }
}

impl Default for Subscriber {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static CURRENT: RefCell<Vec<Arc<SpanContext>>> = RefCell::new(Vec::new());
}

impl tracing_core::Subscriber for Subscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        log::logger().enabled(&as_log(&metadata))
    }

    fn new_span(&self, attrs: &span::Attributes<'_>) -> Id {
        let id = Id::from_u64(self.id_alloc.fetch_add(1, Ordering::SeqCst) as u64);

        self.spans
            .write()
            .insert(id.clone(), Arc::new(SpanContext::new(attrs.metadata())));

        id
    }

    fn record(&self, _: &Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn enter(&self, id: &Id) {
        let context = match self.spans.read().get(id).cloned() {
            Some(context) => context,
            None => panic!("missing context for id: {}", id.into_u64()),
        };

        let logger = log::logger();
        let log_meta = context.log_meta();

        if logger.enabled(&log_meta) {
            self.started_at.write().insert(id.clone(), Instant::now());

            let chain = CURRENT.with(|current| {
                current
                    .borrow()
                    .iter()
                    .map(|c| c.name)
                    .chain(std::iter::once(context.name))
                    .collect::<Vec<_>>()
                    .join(":")
            });

            logger.log(
                &log::Record::builder()
                    .metadata(log_meta)
                    .target(context.target.as_ref())
                    .module_path(context.module_path.as_ref().map(String::as_ref))
                    .file(context.file.as_ref().map(String::as_ref))
                    .line(context.line)
                    .args(format_args!("{:x}: -> {}", id.into_u64(), chain,))
                    .build(),
            )
        }

        CURRENT.with(|current| current.borrow_mut().push(context));
    }

    fn exit(&self, id: &Id) {
        let context = match self.spans.read().get(id).cloned() {
            Some(context) => context,
            None => panic!("missing context for id: {}", id.into_u64()),
        };

        let logger = log::logger();
        let log_meta = context.log_meta();

        if logger.enabled(&log_meta) {
            // calculate the duration spent in the span.
            let duration = match self.started_at.write().remove(id) {
                Some(started_at) => Instant::now().duration_since(started_at),
                None => Duration::default(),
            };

            let chain = CURRENT.with(|current| {
                current
                    .borrow()
                    .iter()
                    .map(|c| c.name)
                    .collect::<Vec<_>>()
                    .join(":")
            });

            logger.log(
                &log::Record::builder()
                    .metadata(log_meta)
                    .target(context.target.as_ref())
                    .module_path(context.module_path.as_ref().map(String::as_ref))
                    .file(context.file.as_ref().map(String::as_ref))
                    .line(context.line)
                    .args(format_args!(
                        "{:x}: <- {} ({:?})",
                        id.into_u64(),
                        chain,
                        duration
                    ))
                    .build(),
            )
        }

        let _ = CURRENT.with(|current| current.borrow_mut().pop());
    }

    fn event(&self, _: &Event<'_>) {
        // ignore
    }

    fn clone_span(&self, id: &Id) -> Id {
        let new_id = Id::from_u64(self.id_alloc.fetch_add(1, Ordering::SeqCst) as u64);

        let context = self.spans.read().get(id).cloned();

        if let Some(context) = context {
            self.spans.write().insert(new_id.clone(), context);
        }

        new_id
    }

    fn try_close(&self, id: Id) -> bool {
        self.spans.write().remove(&id).is_some()
    }
}

/// Helper to convert tracing level to log level.
fn tracing_to_log_level(level: &tracing_core::Level) -> log::Level {
    match *level {
        tracing_core::Level::ERROR => log::Level::Error,
        tracing_core::Level::WARN => log::Level::Warn,
        tracing_core::Level::INFO => log::Level::Info,
        tracing_core::Level::DEBUG => log::Level::Debug,
        tracing_core::Level::TRACE => log::Level::Trace,
    }
}

/// Helper to convert metadata into log metadata.
fn as_log<'a>(meta: &Metadata<'a>) -> log::Metadata<'a> {
    log::Metadata::builder()
        .level(tracing_to_log_level(meta.level()))
        .target(meta.target())
        .build()
}
