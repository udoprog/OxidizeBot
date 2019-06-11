use crate::{auth, command, config, module, prelude::*, template, timer, utils};
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

impl command::Handler for Handler {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Countdown)
    }

    fn handle(&mut self, ctx: &mut command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        match ctx.next() {
            Some("set") => {
                let duration = ctx_try!(ctx.next_parse("<duration> <template>", "!countdown set"));
                let template = ctx_try!(ctx.rest_parse("<duration> <template>", "!countdown set"));

                match self.sender.unbounded_send(Event::Set(duration, template)) {
                    Ok(()) => {
                        ctx.respond("Countdown set!");
                    }
                    Err(_) => {
                        ctx.respond("Could not set countdown :(");
                        return Ok(());
                    }
                }
            }
            Some("clear") => match self.sender.unbounded_send(Event::Clear) {
                Ok(()) => {
                    ctx.respond("Countdown cleared!");
                }
                Err(_) => {
                    ctx.respond("Could not clear countdown :(");
                    return Ok(());
                }
            },
            _ => {
                ctx.respond("Expected: !countdown set <duration> <template>, or !countdown clear");
                return Ok(());
            }
        }

        Ok(())
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    path: Option<PathBuf>,
}

pub struct Module {
    config_path: Option<PathBuf>,
}

impl Module {
    /// Load the given module configuration with backwards compatibility.
    pub fn load(config: &config::Config) -> Self {
        let mut config_path = None;

        for m in &config.modules {
            match *m {
                module::Config::Countdown(ref config) => {
                    log::warn!("`[[modules]] type = \"countdown\"` configuration is deprecated");
                    config_path = config.path.clone();
                }
                _ => (),
            }
        }

        Module { config_path }
    }
}

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "countdown"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            futures,
            settings,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        let settings = settings.scoped("countdown");

        let (mut enabled_stream, enabled) = settings.stream("enabled").or_with(true)?;
        let enabled = Arc::new(RwLock::new(enabled));

        let (mut path_stream, mut path) = settings.stream::<PathBuf>("path").optional()?;

        if let (None, Some(config_path)) = (path.as_ref(), self.config_path.clone()) {
            log::warn!("[countdown] configuration has been deprecated.");
            settings.set("path", &config_path)?;
            path = Some(config_path);
        }

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
                        match out.transpose()? {
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
                                    interval: timer::Interval::new_interval(time::Duration::from_secs(1)),
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
    fn write(&self, timer: &Timer) -> Result<(), failure::Error> {
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

    fn clear(&self) -> Result<(), failure::Error> {
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
            log_err!(e, "failed to write");
        }
    }

    /// Attempt to clear the file and log on errors.
    fn clear_log(&self) {
        if let Err(e) = self.clear() {
            log_err!(e, "failed to clear");
        }
    }
}

struct Timer {
    duration: utils::Duration,
    elapsed: utils::Duration,
    interval: timer::Interval,
}

impl Stream for Timer {
    type Item = Result<(), failure::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(Some(_)) = Pin::new(&mut self.interval).poll_next(cx)? {
            self.as_mut().elapsed += utils::Duration::seconds(1);

            if self.as_ref().elapsed >= self.as_ref().duration {
                return Poll::Ready(None);
            }

            return Poll::Ready(Some(Ok(())));
        }

        Poll::Pending
    }
}

impl stream::FusedStream for Timer {
    fn is_terminated(&self) -> bool {
        self.elapsed >= self.duration
    }
}
