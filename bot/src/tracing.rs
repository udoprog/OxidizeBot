#![allow(unused)]

use std::collections::VecDeque;
use std::io;

use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

pub(crate) fn capture() -> (Capture, Output) {
    (Capture, Output)
}

pub(crate) struct Output;

pub(crate) struct Capture;

#[derive(Default)]
pub(crate) struct LogWriter {
    buf: VecDeque<String>,
}

impl<S> Layer<S> for Capture
where
    S: Subscriber,
{
    fn on_event(&self, e: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let Some(module_path) = e.metadata().module_path() else {
            return;
        };

        // println!("{:?}", e.metadata());
    }
}
