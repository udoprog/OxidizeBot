use crate::{
    auth::Scope,
    command,
    currency::{BalanceTransferError, Currency},
    db,
    injector::Injector,
    prelude::*,
    task, utils,
};
use anyhow::Error;
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
    async fn handle(&mut self, mut ctx: command::Context) -> Result<(), Error> {
        let currency = match self.currency.read().as_ref() {
            Some(currency) => currency.clone(),
            None => {
                ctx.respond("No currency configured");
                return Ok(());
            }
        };

        match ctx.next().as_deref() {
            None => {
                let user = match ctx.user.real() {
                    Some(user) => user,
                    None => {
                        ctx.respond("Only real users can check their balance");
                        return Ok(());
                    }
                };

                let result = currency.balance_of(user.channel(), user.name()).await;

                match result {
                    Ok(balance) => {
                        let balance = balance.unwrap_or_default();
                        let watch_time = utils::compact_duration(&balance.watch_time().as_std());

                        user.respond(format!(
                            "You have {balance} {name} [{watch_time}].",
                            balance = balance.balance,
                            name = currency.name,
                            watch_time = watch_time,
                        ));
                    }
                    Err(e) => {
                        user.respond("Could not get balance, sorry :(");
                        log_error!(e, "failed to get balance");
                    }
                }
            }
            Some("show") => {
                ctx.check_scope(Scope::CurrencyShow).await?;
                let to_show = ctx_try!(ctx.next_str("<user>"));

                match currency.balance_of(ctx.channel(), to_show.as_str()).await {
                    Ok(balance) => {
                        let balance = balance.unwrap_or_default();
                        let watch_time = utils::compact_duration(&balance.watch_time().as_std());

                        ctx.respond(format!(
                            "{user} has {balance} {name} [{watch_time}].",
                            user = to_show,
                            balance = balance.balance,
                            name = currency.name,
                            watch_time = watch_time,
                        ));
                    }
                    Err(e) => {
                        ctx.respond("Count not get balance, sorry :(");
                        log_error!(e, "failed to get balance");
                    }
                }
            }
            Some("give") => {
                let taker = db::user_id(&ctx_try!(ctx.next_str("<user> <amount>")));
                let amount: i64 = ctx_try!(ctx.next_parse("<user> <amount>"));

                let user = match ctx.user.real() {
                    Some(user) => user,
                    None => {
                        ctx.respond("Only real users can give currency");
                        return Ok(());
                    }
                };

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

                let result = currency
                    .balance_transfer(
                        user.channel(),
                        user.name(),
                        &taker,
                        amount,
                        user.is_streamer(),
                    )
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
                        log_error!(e, "failed to modify currency");
                    }
                }
            }
            Some("boost") => {
                ctx.check_scope(Scope::CurrencyBoost).await?;

                let boosted_user = db::user_id(&ctx_try!(ctx.next_str("<user> <amount>")));
                let amount: i64 = ctx_try!(ctx.next_parse("<user> <amount>"));

                if !ctx.user.is_streamer() && ctx.user.is(&boosted_user) {
                    ctx.respond("You gonna have to play by the rules (or ask another mod) :(");
                    return Ok(());
                }

                let user = ctx.user.clone();
                let currency = currency.clone();

                let result = currency
                    .balance_add(user.channel(), &boosted_user, amount)
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
                        log_error!(e, "failed to modify currency");
                    }
                }
            }
            Some("windfall") => {
                ctx.check_scope(Scope::CurrencyWindfall).await?;

                let user = ctx.user.clone();
                let amount: i64 = ctx_try!(ctx.next_parse("<amount>"));
                let sender = ctx.inner.sender.clone();

                task::spawn(async move {
                    let result = currency.add_channel_all(user.channel(), amount, 0).await;

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
                            log_error!(e, "failed to windfall");
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
