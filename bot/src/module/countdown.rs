use std::fs;
use std::path::PathBuf;
use std::pin::pin;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time;

use anyhow::Result;
use async_fuse::Fuse;
use async_trait::async_trait;
use chat::command;
use chat::module;
use common::stream::Stream;
use common::Duration;
use serde::Serialize;
use tokio::sync::mpsc;

enum Event {
    /// Set the countdown.
    Set(Duration, template::Template),
    /// Clear the countdown.
    Clear,
}

pub(crate) struct Handler {
    sender: mpsc::UnboundedSender<Event>,
    enabled: settings::Var<bool>,
}

#[async_trait]
impl command::Handler for Handler {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Countdown)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        match ctx.next().as_deref() {
            Some("set") => {
                let duration = ctx.next_parse("<duration> <template>")?;
                let template = ctx.rest_parse("<duration> <template>")?;

                match self.sender.send(Event::Set(duration, template)) {
                    Ok(()) => {
                        chat::respond!(ctx, "Countdown set!");
                    }
                    Err(_) => {
                        chat::respond!(ctx, "Could not set countdown :(");
                        return Ok(());
                    }
                }
            }
            Some("clear") => match self.sender.send(Event::Clear) {
                Ok(()) => {
                    chat::respond!(ctx, "Countdown cleared!");
                }
                Err(_) => {
                    chat::respond!(ctx, "Could not clear countdown :(");
                    return Ok(());
                }
            },
            _ => {
                chat::respond!(
                    ctx,
                    "Expected: !countdown set <duration> <template>, or !countdown clear"
                );
                return Ok(());
            }
        }

        Ok(())
    }
}

pub(crate) struct Module;

#[async_trait]
impl chat::Module for Module {
    fn ty(&self) -> &'static str {
        "countdown"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            futures,
            settings,
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        let settings = settings.scoped("countdown");

        let (mut enabled_stream, enabled) = settings.stream("enabled").or_with(true).await?;
        let enabled = settings::Var::new(enabled);

        let (mut path_stream, path) = settings.stream::<PathBuf>("path").optional().await?;

        let mut writer = FileWriter::default();
        writer.path = path;

        let (sender, mut receiver) = mpsc::unbounded_channel();

        handlers.insert(
            "countdown",
            Handler {
                sender,
                enabled: enabled.clone(),
            },
        );

        let future = async move {
            let mut timer = pin!(Fuse::empty());

            loop {
                tokio::select! {
                    update = path_stream.recv() => {
                        writer.path = update;
                    }
                    update = enabled_stream.recv() => {
                        if !update {
                            timer.set(Fuse::empty());
                            writer.clear_log();
                        }

                        *enabled.write().await = update;
                    }
                    out = timer.as_mut().poll_stream(Stream::poll_next) => {
                        match out {
                            Some(()) => if let Some(timer) = timer.as_inner_ref() {
                                writer.write_log(timer);
                            },
                            None => {
                                writer.clear_log();
                            },
                        }
                    },
                    Some(event) = receiver.recv() => {
                        match event {
                            Event::Set(duration, template) => {
                                let t = Timer {
                                    duration,
                                    elapsed: Default::default(),
                                    interval: tokio::time::interval(time::Duration::from_secs(1)),
                                };

                                writer.template = Some(template);
                                writer.write_log(&t);
                                timer.set(Fuse::new(t));
                            }
                            Event::Clear => {
                                timer.set(Fuse::empty());
                                writer.clear_log();
                            }
                        }
                    }
                }
            }
        };

        futures.push(Box::pin(future));
        Ok(())
    }
}

#[derive(Default)]
struct FileWriter {
    path: Option<PathBuf>,
    template: Option<template::Template>,
}

impl FileWriter {
    fn write(&self, timer: &Timer) -> Result<()> {
        let path = match &self.path {
            Some(path) => path,
            None => return Ok(()),
        };

        let template = match &self.template {
            Some(template) => template,
            None => return Ok(()),
        };

        tracing::trace!("Writing to log: {}", path.display());

        let mut f = fs::File::create(path)?;
        let remaining = timer.duration.saturating_sub(timer.elapsed);
        let remaining = remaining.as_digital();
        let elapsed = timer.elapsed.as_digital();
        let duration = timer.duration.as_digital();

        template.render(
            &mut f,
            Data {
                remaining,
                elapsed,
                duration,
            },
        )?;

        return Ok(());

        #[derive(Serialize)]
        struct Data {
            remaining: String,
            elapsed: String,
            duration: String,
        }
    }

    fn clear(&self) -> Result<()> {
        let path = match &self.path {
            Some(path) => path,
            None => return Ok(()),
        };

        tracing::trace!("Clearing log: {}", path.display());

        if !path.is_file() {
            return Ok(());
        }

        fs::remove_file(path)?;
        Ok(())
    }

    /// Attempt to write an update and log on errors.
    fn write_log(&self, timer: &Timer) {
        if let Err(e) = self.write(timer) {
            common::log_error!(e, "Failed to write");
        }
    }

    /// Attempt to clear the file and log on errors.
    fn clear_log(&self) {
        if let Err(e) = self.clear() {
            common::log_error!(e, "Failed to clear");
        }
    }
}

struct Timer {
    duration: Duration,
    elapsed: Duration,
    interval: tokio::time::Interval,
}

impl Stream for Timer {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.as_mut().interval).poll_tick(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(..) => (),
        }

        self.as_mut().elapsed += Duration::seconds(1);

        if self.as_ref().elapsed >= self.as_ref().duration {
            return Poll::Ready(None);
        }

        Poll::Ready(Some(()))
    }
}
