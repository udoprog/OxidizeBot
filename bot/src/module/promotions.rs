use crate::{command, db, irc, module, utils};
use chrono::Utc;
use futures::{future, Future as _, Stream as _};
use std::{sync::Arc, time};
use tokio_timer::Interval;

pub struct Handler {
    pub promotions: db::Promotions,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        match ctx.next() {
            Some("list") => {
                let mut names = self
                    .promotions
                    .list(ctx.user.target)
                    .into_iter()
                    .map(|c| c.key.name.to_string())
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    ctx.respond("No custom promotions.");
                } else {
                    names.sort();
                    ctx.respond(format!("Custom promotions: {}", names.join(", ")));
                }
            }
            Some("edit") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                let frequency = match ctx.next() {
                    Some(frequency) => match utils::parse_duration(frequency)
                        .map_err(|_| ())
                        .and_then(|d| chrono::Duration::from_std(d).map_err(|_| ()))
                    {
                        Ok(frequency) => frequency,
                        Err(()) => {
                            ctx.respond(format!("Bad <frequency>: {}", frequency));
                            return Ok(());
                        }
                    },
                    None => {
                        ctx.respond("Expected frequency.");
                        return Ok(());
                    }
                };

                self.promotions
                    .edit(ctx.user.target, name, frequency, ctx.rest())?;
                ctx.respond("Edited promo.");
            }
            Some("delete") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond("Expected name.");
                        return Ok(());
                    }
                };

                if self.promotions.delete(ctx.user.target, name)? {
                    ctx.respond(format!("Deleted promo `{}`.", name));
                } else {
                    ctx.respond("No such promo.");
                }
            }
            Some("rename") => {
                ctx.check_moderator()?;

                let (from, to) = match (ctx.next(), ctx.next()) {
                    (Some(from), Some(to)) => (from, to),
                    _ => {
                        ctx.respond("Expected: !promo rename <from> <to>");
                        return Ok(());
                    }
                };

                match self.promotions.rename(ctx.user.target, from, to) {
                    Ok(()) => ctx.respond(format!("Renamed promo {} -> {}", from, to)),
                    Err(db::RenameError::Conflict) => {
                        ctx.respond(format!("Already a promo named {}", to))
                    }
                    Err(db::RenameError::Missing) => {
                        ctx.respond(format!("No such promo: {}", from))
                    }
                }
            }
            None | Some(..) => {
                ctx.respond("Expected: list, edit, or delete.");
            }
        }

        Ok(())
    }
}

pub struct Module {
    frequency: time::Duration,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(
        default = "default_duration",
        deserialize_with = "utils::deserialize_duration"
    )]
    frequency: time::Duration,
}

fn default_duration() -> time::Duration {
    time::Duration::from_secs(5 * 60)
}

impl Module {
    pub fn load(config: &Config) -> Result<Self, failure::Error> {
        Ok(Module {
            frequency: config.frequency.clone(),
        })
    }
}

impl super::Module for Module {
    fn hook(
        &self,
        module::HookContext {
            handlers,
            promotions,
            futures,
            sender,
            irc_config,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        handlers.insert(
            "promo",
            Handler {
                promotions: promotions.clone(),
            },
        );

        let promotions = promotions.clone();
        let sender = sender.clone();
        let channel = irc_config.channel.to_string();

        let future = Box::new(
            Interval::new_interval(self.frequency.clone())
                .map_err(|_| ())
                .for_each(move |_| {
                    let promotions = promotions.clone();
                    let sender = sender.clone();
                    let channel = channel.clone();

                    tokio::spawn(future::lazy({
                        move || {
                            if let Err(e) = promote(promotions, sender, &channel) {
                                log::error!("failed to run promotions: {}", e);
                            }

                            Ok(())
                        }
                    }))
                })
                .map_err(|()| failure::format_err!("interval timer failed")),
        ) as utils::BoxFuture<(), failure::Error>;

        futures.push(future);

        return Ok(());

        /// Run the next promotion.
        fn promote(
            promotions: db::Promotions,
            sender: irc::Sender,
            channel: &str,
        ) -> Result<(), failure::Error> {
            if let Some(p) = pick(promotions.list(channel)) {
                let text = p.render(&PromoData { channel })?;
                promotions.bump_promoted_at(&*p)?;
                sender.privmsg(channel, text);
            }

            Ok(())
        }

        #[derive(Debug, serde::Serialize)]
        struct PromoData<'a> {
            channel: &'a str,
        }

        /// Pick the best promo.
        fn pick(mut promotions: Vec<Arc<db::Promotion>>) -> Option<Arc<db::Promotion>> {
            promotions.sort_by(|a, b| a.promoted_at.cmp(&b.promoted_at));

            let now = Utc::now();

            for p in promotions {
                let promoted_at = match p.promoted_at.as_ref() {
                    None => return Some(p),
                    Some(promoted_at) => promoted_at,
                };

                if now.clone().signed_duration_since(promoted_at.clone()) < p.frequency {
                    continue;
                }

                return Some(p);
            }

            None
        }
    }
}
