use crate::{auth, command, irc, module, prelude::*, utils};
use chrono::{DateTime, Utc};
use failure::Error;
use hashbrown::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the !poll command.
pub struct Poll {
    enabled: Arc<RwLock<bool>>,
    polls: HashMap<String, ActivePoll>,
}

impl command::Handler for Poll {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Poll)
    }

    fn handle(&mut self, ctx: &mut command::Context<'_, '_>) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        match ctx.next().as_ref().map(String::as_str) {
            Some("run") => {
                let question = ctx_try!(ctx.next_str("<question> <options...>"));

                let mut options = HashMap::new();

                while let Some(option) = ctx.next() {
                    let (key, description) = match option.find('=') {
                        Some(i) => {
                            let (keyword, description) = option.split_at(i);
                            (keyword.to_string(), Some(description[1..].to_string()))
                        }
                        None => (option, None),
                    };

                    options.insert(key.to_lowercase(), description);
                }

                let poll = ActivePoll {
                    inner: Arc::new(RwLock::new(Inner {
                        voted: Default::default(),
                        votes: Default::default(),
                        options,
                        created_at: Utc::now(),
                    })),
                };

                ctx.message_hooks
                    .insert(format!("poll/{}", question), Box::new(poll.clone()));
                self.polls.insert(question.clone(), poll);

                ctx.respond(format!("Started poll `{}`", question));
            }
            Some("close") => {
                let question = match ctx.next() {
                    Some(question) => question,
                    None => {
                        let latest = self
                            .polls
                            .iter()
                            .max_by_key(|e| e.1.inner.read().created_at.clone());

                        match latest {
                            Some((question, _)) => question.to_string(),
                            None => {
                                ctx.respond("No running polls");
                                return Ok(());
                            }
                        }
                    }
                };

                let mut poll = match self.polls.remove(&question) {
                    Some(poll) => poll,
                    None => {
                        ctx.respond(format!("No poll named `{}`!", question));
                        return Ok(());
                    }
                };

                ctx.message_hooks.remove(&format!("poll/{}", question));
                let results = poll.close();

                let total = results.iter().map(|(_, c)| c).sum::<u32>();

                let mut formatted = Vec::new();

                for (key, votes) in results {
                    let p = utils::percentage(votes, total);

                    let votes = match votes {
                        0 => format!("no votes"),
                        1 => format!("one vote"),
                        n => format!("{} votes", n),
                    };

                    formatted.push(format!("{} = {} ({})", key, votes, p));
                }

                ctx.respond(format!("{} -> {}.", question, formatted.join(", ")));
            }
            _ => ctx.respond("Expected: run, close."),
        }

        return Ok(());
    }
}

struct Inner {
    voted: HashSet<String>,
    votes: HashMap<String, u32>,
    options: HashMap<String, Option<String>>,
    created_at: DateTime<Utc>,
}

#[derive(Clone)]
struct ActivePoll {
    inner: Arc<RwLock<Inner>>,
}

impl ActivePoll {
    /// Close the poll.
    pub fn close(&mut self) -> Vec<(String, u32)> {
        let inner = self.inner.read();

        let mut results = Vec::new();

        for (o, description) in &inner.options {
            results.push((
                description.clone().unwrap_or_else(|| o.to_string()),
                inner.votes.get(o).cloned().unwrap_or_default(),
            ));
        }

        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }
}

impl command::MessageHook for ActivePoll {
    fn peek(&mut self, user: &irc::User<'_>, m: &str) -> Result<(), Error> {
        let mut inner = self.inner.write();

        if inner.voted.contains(user.name) {
            return Ok(());
        }

        for word in utils::TrimmedWords::new(m) {
            if inner.options.get(&word.to_lowercase()).is_none() {
                continue;
            }

            *inner.votes.entry(word.to_string()).or_default() += 1;
            inner.voted.insert(user.name.to_string());
            break;
        }

        Ok(())
    }
}

pub struct Module;

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "poll"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            futures,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), failure::Error> {
        let mut vars = settings.vars();

        handlers.insert(
            "poll",
            Poll {
                polls: Default::default(),
                enabled: vars.var("poll/enabled", false)?,
            },
        );

        futures.push(vars.run().boxed());
        Ok(())
    }
}
