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
pub struct Handler<'a> {
    pub currency: Arc<RwLock<Option<Currency>>>,
    pub db: &'a db::Database,
}

impl Handler<'_> {
    /// Get the name of the command for the current currency.
    pub fn command_name(&self) -> Option<Arc<String>> {
        let currency = self.currency.read();

        match currency.as_ref() {
            Some(ref c) if c.command_enabled => Some(c.name.clone()),
            _ => None,
        }
    }
}

impl command::Handler for Handler<'_> {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), Error> {
        let currency = self.currency.read();
        let currency = match currency.as_ref() {
            Some(currency) => currency.clone(),
            None => {
                ctx.respond("No currency configured");
                return Ok(());
            }
        };

        match ctx.next() {
            None => {
                let user = ctx.user.as_owned_user();

                ctx.spawn(async move {
                    let result = currency
                        .balance_of(user.target.clone(), user.name.clone())
                        .await;

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
                let to_show = ctx_try!(ctx.next_str("<user>", "!currency show")).to_string();

                let user = ctx.user.as_owned_user();

                ctx.spawn(async move {
                    let result = currency
                        .balance_of(user.target.clone(), to_show.clone())
                        .await;

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
                let taker =
                    db::user_id(ctx_try!(ctx.next_str("<user> <amount>", "!currency give")));
                let amount: i64 = ctx_try!(ctx.next_parse("<user> <amount>", "!currency give"));

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

                let target = ctx.user.target.to_owned();
                let giver = ctx.user.as_owned_user();
                let is_streamer = ctx.user.is(ctx.streamer);

                ctx.spawn(async move {
                    let result = currency
                        .balance_transfer(
                            target,
                            giver.name.clone(),
                            taker.clone(),
                            amount,
                            is_streamer,
                        )
                        .await;

                    match result {
                        Ok(()) => {
                            giver.respond(format!(
                                "Gave {user} {amount} {currency}!",
                                user = taker,
                                amount = amount,
                                currency = currency.name
                            ));
                        }
                        Err(BalanceTransferError::NoBalance) => {
                            giver.respond(format!(
                                "Not enough {currency} to transfer {amount}",
                                currency = currency.name,
                                amount = amount,
                            ));
                        }
                        Err(BalanceTransferError::Other(e)) => {
                            giver.respond(format!(
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

                let boosted_user =
                    db::user_id(ctx_try!(ctx.next_str("<user> <amount>", "!currency boost")));
                let amount: i64 = ctx_try!(ctx.next_parse("<user> <amount>", "!currency boost"));

                if !ctx.user.is(ctx.streamer) && ctx.user.is(&boosted_user) {
                    ctx.respond("You gonna have to play by the rules (or ask another mod) :(");
                    return Ok(());
                }

                let user = ctx.user.as_owned_user();
                let currency = currency.clone();

                ctx.spawn(async move {
                    let result = currency
                        .balance_add(user.target.clone(), boosted_user.clone(), amount)
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

                let user = ctx.user.as_owned_user();
                let amount: i64 = ctx_try!(ctx.next_parse("<amount>", "!currency windfall"));
                let sender = ctx.sender.clone();

                ctx.spawn(async move {
                    let result = currency.add_channel_all(user.target.clone(), amount).await;

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

                if ctx.has_scope(Scope::CurrencyBoost) {
                    alts.push("boost");
                }

                if ctx.has_scope(Scope::CurrencyWindfall) {
                    alts.push("windfall");
                }

                if ctx.has_scope(Scope::CurrencyShow) {
                    alts.push("show");
                }

                ctx.respond(format!("Expected: {alts}", alts = alts.join(", ")));
            }
        }

        Ok(())
    }
}

pub fn setup<'a>(
    injector: &Injector,
    db: &'a db::Database,
) -> Result<(impl Future<Output = Result<(), Error>> + 'a, Handler<'a>), Error> {
    let (currency_stream, currency) = injector.stream::<Currency>();
    let currency = Arc::new(RwLock::new(currency));

    let handler = Handler {
        db,
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
