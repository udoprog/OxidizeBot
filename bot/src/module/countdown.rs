use crate::{command, config, module, template, utils};
use failure::format_err;
use futures::{sync::mpsc, Async, Future, Poll, Stream};
use std::{fs, path::PathBuf, time};
use tokio::timer;

enum Event {
    /// Set the countdown.
    Set(time::Duration, template::Template),
    /// Clear the countdown.
    Clear,
}

pub struct Handler {
    sender: mpsc::UnboundedSender<Event>,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        ctx.check_moderator()?;

        match ctx.next() {
            Some("set") => {
                let duration = match ctx.next() {
                    Some(duration) => match utils::parse_duration(duration) {
                        Ok(duration) => duration,
                        Err(_) => {
                            ctx.respond("Countdown not added, Bad <duration> :(");
                            return Ok(());
                        }
                    },
                    None => {
                        ctx.respond("Usage: !countdown <duration> <template>");
                        return Ok(());
                    }
                };

                let template = ctx.rest();

                let template = match template::Template::compile(template) {
                    Ok(template) => template,
                    Err(_) => {
                        ctx.respond("Countdown not added, bad <template> :(");
                        return Ok(());
                    }
                };

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

pub struct Countdown {
    path: PathBuf,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    path: PathBuf,
}

impl Countdown {
    pub fn load(_config: &config::Config, module: &Config) -> Result<Self, failure::Error> {
        Ok(Countdown {
            path: module.path.clone(),
        })
    }
}

impl super::Module for Countdown {
    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers, futures, ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        let (sender, receiver) = mpsc::unbounded();

        handlers.insert("countdown", Handler { sender });

        futures.push(Box::new(CountdownFuture {
            receiver,
            path: self.path.clone(),
            current: None,
        }));

        Ok(())
    }
}

struct CountdownFuture {
    receiver: mpsc::UnboundedReceiver<Event>,
    path: PathBuf,
    current: Option<Current>,
}

struct Current {
    duration: time::Duration,
    template: template::Template,
    elapsed: time::Duration,
    interval: timer::Interval,
    path: PathBuf,
}

impl Current {
    fn write(&mut self) -> Result<(), failure::Error> {
        let mut f = fs::File::create(&self.path)?;
        let remaining = self
            .duration
            .checked_sub(self.elapsed.clone())
            .unwrap_or_default();
        let remaining = utils::digital_duration(&remaining);
        let elapsed = utils::digital_duration(&self.elapsed);
        let duration = utils::digital_duration(&self.duration);

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

    fn clear(&mut self) -> Result<(), failure::Error> {
        if !self.path.is_file() {
            return Ok(());
        }

        fs::remove_file(&self.path)?;
        Ok(())
    }

    /// Attempt to write an update and log on errors.
    fn write_log(&mut self) {
        if let Err(e) = self.write() {
            utils::log_err(format!("failed to write: {}", self.path.display()), e);
        }
    }

    /// Attempt to clear the file and log on errors.
    fn clear_log(&mut self) {
        if let Err(e) = self.clear() {
            utils::log_err(format!("failed to clear: {}", self.path.display()), e);
        }
    }
}

impl Stream for Current {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<()>, failure::Error> {
        match self.interval.poll()? {
            Async::Ready(None) => failure::bail!("interval timer ended"),
            Async::Ready(Some(_)) => {
                self.elapsed += time::Duration::from_secs(1);

                if self.elapsed >= self.duration {
                    return Ok(Async::Ready(None));
                }

                self.write()?;
                Ok(Async::Ready(Some(())))
            }
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}

impl Future for CountdownFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<(), failure::Error> {
        loop {
            let mut not_ready = true;

            match self
                .receiver
                .poll()
                .map_err(|_| format_err!("failed to poll receiver"))?
            {
                Async::Ready(None) => failure::bail!("receiver queue ended"),
                Async::Ready(Some(e)) => {
                    match e {
                        Event::Set(duration, template) => {
                            let mut current = Current {
                                duration,
                                template,
                                elapsed: Default::default(),
                                interval: timer::Interval::new_interval(time::Duration::from_secs(
                                    1,
                                )),
                                path: self.path.clone(),
                            };

                            current.write_log();
                            self.current = Some(current);
                        }
                        Event::Clear => {
                            if let Some(mut current) = self.current.take() {
                                current.clear_log();
                            }
                        }
                    }

                    not_ready = false;
                }
                Async::NotReady => (),
            }

            if let Some(current) = self.current.as_mut() {
                match current.poll()? {
                    Async::Ready(None) => {
                        current.clear_log();
                        self.current = None;
                    }
                    Async::Ready(Some(())) => not_ready = false,
                    Async::NotReady => (),
                }
            }

            if not_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}
