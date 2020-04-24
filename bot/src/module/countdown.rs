use crate::{auth, command, module, prelude::*, template, utils};
use parking_lot::RwLock;
use std::{fs, path::PathBuf, sync::Arc, time};

enum Event {
    /// Set the countdown.
    Set(utils::Duration, template::Template),
    /// Clear the countdown.
    Clear,
}

pub struct Handler {
    sender: mpsc::UnboundedSender<Event>,
    enabled: Arc<RwLock<bool>>,
}

#[async_trait]
impl command::Handler for Handler {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Countdown)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<(), anyhow::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        match ctx.next().as_deref() {
            Some("set") => {
                let duration = ctx.next_parse("<duration> <template>")?;
                let template = ctx.rest_parse("<duration> <template>")?;

                match self.sender.unbounded_send(Event::Set(duration, template)) {
                    Ok(()) => {
                        respond!(ctx, "Countdown set!");
                    }
                    Err(_) => {
                        respond!(ctx, "Could not set countdown :(");
                        return Ok(());
                    }
                }
            }
            Some("clear") => match self.sender.unbounded_send(Event::Clear) {
                Ok(()) => {
                    respond!(ctx, "Countdown cleared!");
                }
                Err(_) => {
                    respond!(ctx, "Could not clear countdown :(");
                    return Ok(());
                }
            },
            _ => {
                respond!(
                    ctx,
                    "Expected: !countdown set <duration> <template>, or !countdown clear"
                );
                return Ok(());
            }
        }

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
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
    ) -> Result<(), anyhow::Error> {
        let settings = settings.scoped("countdown");

        let (mut enabled_stream, enabled) = settings.stream("enabled").or_with(true).await?;
        let enabled = Arc::new(RwLock::new(enabled));

        let (mut path_stream, path) = settings.stream::<PathBuf>("path").optional().await?;

        let mut writer = FileWriter::default();
        writer.path = path;

        let (sender, mut receiver) = mpsc::unbounded();

        handlers.insert(
            "countdown",
            Handler {
                sender,
                enabled: enabled.clone(),
            },
        );

        let future = async move {
            let mut timer = Option::<Timer>::None;

            loop {
                futures::select! {
                    update = path_stream.select_next_some() => {
                        writer.path = update;
                    }
                    update = enabled_stream.select_next_some() => {
                        if (!update) {
                            timer.take();
                            writer.clear_log();
                        }

                        *enabled.write() = update;
                    }
                    out = timer.next() => {
                        match out {
                            Some(()) => if let Some(timer) = timer.as_ref() {
                                writer.write_log(timer);
                            },
                            None => {
                                writer.clear_log();
                            },
                        }
                    },
                    event = receiver.select_next_some() => {
                        match event {
                            Event::Set(duration, template) => {
                                let mut t = Timer {
                                    duration,
                                    elapsed: Default::default(),
                                    interval: tokio::time::interval(time::Duration::from_secs(1)),
                                };

                                writer.template = Some(template);
                                writer.write_log(&t);
                                timer = Some(t);
                            }
                            Event::Clear => {
                                timer.take();
                                writer.clear_log();
                            }
                        }
                    }
                }
            }
        };

        futures.push(future.boxed());
        Ok(())
    }
}

#[derive(Default)]
struct FileWriter {
    path: Option<PathBuf>,
    template: Option<template::Template>,
}

impl FileWriter {
    fn write(&self, timer: &Timer) -> Result<(), anyhow::Error> {
        let path = match &self.path {
            Some(path) => path,
            None => return Ok(()),
        };

        let template = match &self.template {
            Some(template) => template,
            None => return Ok(()),
        };

        log::trace!("Writing to log: {}", path.display());

        let mut f = fs::File::create(path)?;
        let remaining = timer.duration.saturating_sub(timer.elapsed.clone());
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

        #[derive(serde::Serialize)]
        struct Data {
            remaining: String,
            elapsed: String,
            duration: String,
        }
    }

    fn clear(&self) -> Result<(), anyhow::Error> {
        let path = match &self.path {
            Some(path) => path,
            None => return Ok(()),
        };

        log::trace!("Clearing log: {}", path.display());

        if !path.is_file() {
            return Ok(());
        }

        fs::remove_file(path)?;
        Ok(())
    }

    /// Attempt to write an update and log on errors.
    fn write_log(&self, timer: &Timer) {
        if let Err(e) = self.write(timer) {
            log_error!(e, "failed to write");
        }
    }

    /// Attempt to clear the file and log on errors.
    fn clear_log(&self) {
        if let Err(e) = self.clear() {
            log_error!(e, "failed to clear");
        }
    }
}

struct Timer {
    duration: utils::Duration,
    elapsed: utils::Duration,
    interval: tokio::time::Interval,
}

impl Stream for Timer {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(Some(_)) = Pin::new(&mut self.interval).poll_next(cx) {
            self.as_mut().elapsed += utils::Duration::seconds(1);

            if self.as_ref().elapsed >= self.as_ref().duration {
                return Poll::Ready(None);
            }

            return Poll::Ready(Some(()));
        }

        Poll::Pending
    }
}

impl stream::FusedStream for Timer {
    fn is_terminated(&self) -> bool {
        self.elapsed >= self.duration
    }
}
