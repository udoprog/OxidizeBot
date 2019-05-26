use crate::{command, config, module, prelude::*, template, timer, utils};
use parking_lot::RwLock;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time,
};

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
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        match ctx.next() {
            Some("set") => {
                ctx.check_moderator()?;

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
            Some("clear") => {
                ctx.check_moderator()?;

                match self.sender.unbounded_send(Event::Clear) {
                    Ok(()) => {
                        ctx.respond("Countdown cleared!");
                    }
                    Err(_) => {
                        ctx.respond("Could not clear countdown :(");
                        return Ok(());
                    }
                }
            }
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
    pub fn load(_config: &config::Config, module: &Config) -> Result<Self, failure::Error> {
        Ok(Module {
            config_path: module.path.clone(),
        })
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
        let settings = settings.scoped(&["countdown"]);

        let (mut enabled_stream, enabled) = settings.init_and_stream("enabled", true)?;
        let enabled = Arc::new(RwLock::new(enabled));

        let (mut path_stream, mut path) = settings.init_and_option_stream::<PathBuf>("path")?;

        if let (None, Some(config_path)) = (path.as_ref(), self.config_path.clone()) {
            log::warn!("[countdown] configuration has been deprecated.");
            settings.set("path", &config_path)?;
        }

        let (sender, mut receiver) = mpsc::unbounded();

        handlers.insert(
            "countdown",
            Handler {
                sender,
                enabled: enabled.clone(),
            },
        );

        let future = async move {
            let mut current = Option::<Current>::None;

            loop {
                futures::select! {
                    update = path_stream.select_next_some() => {
                        path = update;
                    }
                    update = enabled_stream.select_next_some() => {
                        if (!update) {

                        }

                        *enabled.write() = update;
                    }
                    out = current.next() => {
                        let path = match path.as_ref() {
                            Some(path) => path,
                            None => continue,
                        };

                        match out.transpose()? {
                            Some(()) => if let Some(c) = current.as_mut() {
                                c.write_log(path);
                            },
                            None => if let Some(mut c) = current.take() {
                                c.clear_log(path);
                            },
                        }
                    },
                    event = receiver.select_next_some() => {
                        let path = match path.as_ref() {
                            Some(path) => path,
                            None => {
                                log::warn!("countdown/path: not configured");
                                continue;
                            },
                        };

                        match event {
                            Event::Set(duration, template) => {
                                let mut c = Current {
                                    duration,
                                    template,
                                    elapsed: Default::default(),
                                    interval: timer::Interval::new_interval(time::Duration::from_secs(1)),
                                };

                                c.write_log(path);
                                current = Some(c);
                            }
                            Event::Clear => {
                                if let Some(mut c) = current.take() {
                                    c.clear_log(path);
                                }
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

struct Current {
    duration: utils::Duration,
    template: template::Template,
    elapsed: utils::Duration,
    interval: timer::Interval,
}

impl Current {
    fn write(&mut self, path: &Path) -> Result<(), failure::Error> {
        let mut f = fs::File::create(path)?;
        let remaining = self.duration.saturating_sub(self.elapsed.clone());
        let remaining = remaining.as_digital();
        let elapsed = self.elapsed.as_digital();
        let duration = self.duration.as_digital();

        self.template.render(
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

    fn clear(&mut self, path: &Path) -> Result<(), failure::Error> {
        if !path.is_file() {
            return Ok(());
        }

        fs::remove_file(path)?;
        Ok(())
    }

    /// Attempt to write an update and log on errors.
    fn write_log(&mut self, path: &Path) {
        if let Err(e) = self.write(path) {
            log_err!(e, "failed to write: {}", path.display());
        }
    }

    /// Attempt to clear the file and log on errors.
    fn clear_log(&mut self, path: &Path) {
        if let Err(e) = self.clear(path) {
            log_err!(e, "failed to clear: {}", path.display());
        }
    }
}

impl Stream for Current {
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

impl stream::FusedStream for Current {
    fn is_terminated(&self) -> bool {
        self.elapsed >= self.duration
    }
}
