use crate::{
    auth::Scope,
    command,
    currency::{BalanceTransferError, Currency},
    db,
    injector::Injector,
    prelude::*,
};
use failure::Error;
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the !admin command.
pub struct Handler {
    pub currency: Arc<RwLock<Option<Currency>>>,
}

impl Handler {
    /// Get the name of the command for the current currency.
    pub fn command_name(&self) -> Option<Arc<String>> {
        let currency = self.currency.read();

        match currency.as_ref() {
            Some(ref c) if c.command_enabled => Some(c.name.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&mut self, mut ctx: command::Context<'_>) -> Result<(), Error> {
        let currency = self.currency.read();
        let currency = match currency.as_ref() {
            Some(currency) => currency.clone(),
            None => {
                ctx.respond("No currency configured");
                return Ok(());
            }
        };

        match ctx.next().as_ref().map(String::as_str) {
            None => {
                let user = ctx.user.clone();

                ctx.spawn(async move {
                    let result = currency.balance_of(user.target(), user.name()).await;

                    match result {
                        Ok(balance) => {
                            let balance = balance.unwrap_or_default();

                            user.respond(format!(
                                "You have {balance} {name}.",
                                balance = balance,
                                name = currency.name
                            ));
                        }
                        Err(e) => {
                            user.respond("Count not get balance, sorry :(");
                            log_err!(e, "failed to get balance");
                        }
                    }
                });
            }
            Some("show") => {
                ctx.check_scope(Scope::CurrencyShow)?;
                let to_show = ctx_try!(ctx.next_str("<user>"));

                let user = ctx.user.clone();

                ctx.spawn(async move {
                    let result = currency.balance_of(user.target(), to_show.as_str()).await;

                    match result {
                        Ok(balance) => {
                            let balance = balance.unwrap_or_default();

                            user.respond(format!(
                                "{user} has {balance} {name}.",
                                user = to_show,
                                balance = balance,
                                name = currency.name
                            ));
                        }
                        Err(e) => {
                            user.respond("Count not get balance, sorry :(");
                            log_err!(e, "failed to get balance");
                        }
                    }
                });
            }
            Some("give") => {
                let taker = db::user_id(&ctx_try!(ctx.next_str("<user> <amount>")));
                let amount: i64 = ctx_try!(ctx.next_parse("<user> <amount>"));

                if ctx.user.is(&taker) {
                    ctx.respond("Giving to... yourself? But WHY?");
                    return Ok(());
                }

                if amount <= 0 {
                    ctx.respond(format!(
                        "Can't give negative or zero {currency} LUL",
                        currency = currency.name
                    ));
                    return Ok(());
                }

                let user = ctx.user.clone();
                let is_streamer = ctx.user.is_streamer();

                ctx.spawn(async move {
                    let result = currency
                        .balance_transfer(user.target(), user.name(), &taker, amount, is_streamer)
                        .await;

                    match result {
                        Ok(()) => {
                            user.respond(format!(
                                "Gave {user} {amount} {currency}!",
                                user = taker,
                                amount = amount,
                                currency = currency.name
                            ));
                        }
                        Err(BalanceTransferError::NoBalance) => {
                            user.respond(format!(
                                "Not enough {currency} to transfer {amount}",
                                currency = currency.name,
                                amount = amount,
                            ));
                        }
                        Err(BalanceTransferError::Other(e)) => {
                            user.respond(format!(
                                "Failed to give {currency}, sorry :(",
                                currency = currency.name
                            ));
                            log_err!(e, "failed to modify currency");
                        }
                    }
                });
            }
            Some("boost") => {
                ctx.check_scope(Scope::CurrencyBoost)?;

                let boosted_user = db::user_id(&ctx_try!(ctx.next_str("<user> <amount>")));
                let amount: i64 = ctx_try!(ctx.next_parse("<user> <amount>"));

                if !ctx.user.is_streamer() && ctx.user.is(&boosted_user) {
                    ctx.respond("You gonna have to play by the rules (or ask another mod) :(");
                    return Ok(());
                }

                let user = ctx.user.clone();
                let currency = currency.clone();

                ctx.spawn(async move {
                    let result = currency
                        .balance_add(user.target(), &boosted_user, amount)
                        .await;

                    match result {
                        Ok(()) => {
                            if amount >= 0 {
                                user.respond(format!(
                                    "Gave {user} {amount} {currency}!",
                                    user = boosted_user,
                                    amount = amount,
                                    currency = currency.name
                                ));
                            } else {
                                user.respond(format!(
                                    "Took away {amount} {currency} from {user}!",
                                    user = boosted_user,
                                    amount = -amount,
                                    currency = currency.name
                                ));
                            }
                        }
                        Err(e) => {
                            user.respond("failed to boost user, sorry :(");
                            log_err!(e, "failed to modify currency");
                        }
                    }
                });
            }
            Some("windfall") => {
                ctx.check_scope(Scope::CurrencyWindfall)?;

                let user = ctx.user.clone();
                let amount: i64 = ctx_try!(ctx.next_parse("<amount>"));
                let sender = ctx.sender.clone();

                ctx.spawn(async move {
                    let result = currency.add_channel_all(user.target(), amount).await;

                    match result {
                        Ok(_) => {
                            if amount >= 0 {
                                sender.privmsg(format!(
                                    "/me gave {amount} {currency} to EVERYONE!",
                                    amount = amount,
                                    currency = currency.name
                                ));
                            } else {
                                sender.privmsg(format!(
                                    "/me took away {amount} {currency} from EVERYONE!",
                                    amount = amount,
                                    currency = currency.name
                                ));
                            }
                        }
                        Err(e) => {
                            user.respond("failed to windfall :(");
                            log_err!(e, "failed to windfall");
                        }
                    }
                });
            }
            Some(..) => {
                let mut alts = Vec::new();

                alts.push("give");

                if ctx.user.has_scope(Scope::CurrencyBoost) {
                    alts.push("boost");
                }

                if ctx.user.has_scope(Scope::CurrencyWindfall) {
                    alts.push("windfall");
                }

                if ctx.user.has_scope(Scope::CurrencyShow) {
                    alts.push("show");
                }

                ctx.respond(format!("Expected: {alts}", alts = alts.join(", ")));
            }
        }

        Ok(())
    }
}

pub fn setup(
    injector: &Injector,
) -> Result<(impl Future<Output = Result<(), Error>>, Handler), Error> {
    let (currency_stream, currency) = injector.stream::<Currency>();
    let currency = Arc::new(RwLock::new(currency));

    let handler = Handler {
        currency: currency.clone(),
    };

    let future = async move {
        let mut currency_stream = currency_stream.fuse();

        loop {
            futures::select! {
                update = currency_stream.select_next_some() => {
                    *currency.write() = update;
                }
            }
        }
    };

    Ok((future, handler))
}
