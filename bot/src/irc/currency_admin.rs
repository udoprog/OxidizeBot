use crate::{command, currency, db, utils};
use futures::Future as _;

/// Handler for the !admin command.
pub struct Handler {
    pub currency: currency::Currency,
    pub db: db::Database,
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        match ctx.next() {
            None => {
                let balance = self
                    .db
                    .balance_of(ctx.user.target, ctx.user.name)?
                    .unwrap_or(0);

                ctx.respond(format!(
                    "You have {balance} {name}.",
                    balance = balance,
                    name = self.currency.name
                ));
            }
            Some("show") => {
                ctx.check_moderator()?;

                let user = match ctx.next() {
                    Some(user) => user,
                    None => {
                        ctx.respond(format!(
                            "expected {c} <user> to show currency for the given user",
                            c = ctx.alias.unwrap_or("!currency show"),
                        ));
                        return Ok(());
                    }
                };

                let balance = self.db.balance_of(ctx.user.target, user)?.unwrap_or(0);

                ctx.respond(format!(
                    "{user} has {balance} {name}.",
                    user = user,
                    balance = balance,
                    name = self.currency.name
                ));
            }
            Some("give") => {
                let taker = match ctx.next() {
                    Some(taker) => taker,
                    None => {
                        ctx.respond(format!(
                            "expected {c} <user> <amount> to give {currency} to another viewer!",
                            c = ctx.alias.unwrap_or("!currency give"),
                            currency = self.currency.name,
                        ));
                        return Ok(());
                    }
                };

                let amount: i32 = match ctx.next().map(str::parse) {
                    Some(Ok(amount)) => amount,
                    None | Some(Err(_)) => {
                        ctx.respond(format!(
                            "expected {c} <user> <amount> to give {currency} to another viewer!",
                            c = ctx.alias.unwrap_or("!currency give"),
                            currency = self.currency.name,
                        ));
                        return Ok(());
                    }
                };

                if ctx.user.is(taker) {
                    ctx.respond("Giving to... yourself? But WHY?");
                    return Ok(());
                }

                if amount <= 0 {
                    ctx.respond(format!(
                        "Can't give negative or zero {currency} LUL",
                        currency = self.currency.name
                    ));
                    return Ok(());
                }

                let is_streamer = ctx.user.is(ctx.streamer);

                let currency = self.currency.clone();

                let transfer = self.db.balance_transfer(
                    &ctx.user.target,
                    &ctx.user.name,
                    taker,
                    amount,
                    is_streamer,
                );

                let giver = ctx.user.as_owned_user();
                let taker = taker.to_string();

                let future = transfer.then(move |r| match r {
                    Ok(()) => {
                        giver.respond(format!(
                            "Gave {user} {amount} {currency}!",
                            user = taker,
                            amount = amount,
                            currency = currency.name
                        ));

                        Ok(())
                    }
                    Err(db::BalanceTransferError::NoBalance) => {
                        giver.respond(format!(
                            "Not enough {currency} to transfer {amount}",
                            currency = currency.name,
                            amount = amount,
                        ));
                        Ok(())
                    }
                    Err(db::BalanceTransferError::Other(e)) => {
                        giver.respond(format!(
                            "Failed to give {currency}, sorry :(",
                            currency = currency.name
                        ));
                        utils::log_err("failed to modify currency: {}", e);
                        Ok(())
                    }
                });

                ctx.spawn(future);
            }
            Some("boost") => {
                ctx.check_moderator()?;

                let boosted_user = match ctx.next() {
                    Some(boosted_user) => boosted_user.to_string(),
                    None => {
                        ctx.respond(format!(
                            "expected {c} <user> <amount>",
                            c = ctx.alias.unwrap_or("!currency boost")
                        ));
                        return Ok(());
                    }
                };

                if !ctx.user.is(ctx.streamer) && ctx.user.is(&boosted_user) {
                    ctx.respond("You gonna have to play by the rules (or ask another mod) :(");
                    return Ok(());
                }

                let amount = match ctx.next().map(str::parse) {
                    Some(Ok(amount)) => amount,
                    None | Some(Err(_)) => {
                        ctx.respond(format!(
                            "expected {c} <user> <amount>",
                            c = ctx.alias.unwrap_or("!currency boost")
                        ));
                        return Ok(());
                    }
                };

                let user = ctx.user.as_owned_user();
                let currency = self.currency.clone();

                ctx.spawn(
                    self.db
                        .balance_add(&user.target, &boosted_user, amount)
                        .then(move |r| match r {
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

                                Ok(())
                            }
                            Err(e) => {
                                user.respond("failed to boost user, sorry :(");
                                utils::log_err("failed to modify currency: {}", e);
                                Ok(())
                            }
                        }),
                );
            }
            Some("windfall") => {
                ctx.check_moderator()?;

                let amount = match ctx.next().map(str::parse) {
                    Some(Ok(amount)) => amount,
                    None | Some(Err(_)) => {
                        ctx.respond(format!(
                            "expected {c} <user> <amount>",
                            c = ctx.alias.unwrap_or("!currency boost")
                        ));
                        return Ok(());
                    }
                };

                ctx.spawn(
                    self.currency
                        .add_channel_all(&ctx.user.target, amount)
                        .then({
                            let sender = ctx.sender.clone();
                            let currency = self.currency.clone();
                            let user = ctx.user.as_owned_user();

                            move |r| match r {
                                Ok(_) => {
                                    if amount >= 0 {
                                        sender.privmsg(
                                            &user.target,
                                            format!(
                                                "/me gave {amount} {currency} to EVERYONE!",
                                                amount = amount,
                                                currency = currency.name
                                            ),
                                        );
                                    } else {
                                        sender.privmsg(
                                            &user.target,
                                            format!(
                                                "/me took away {amount} {currency} from EVERYONE!",
                                                amount = amount,
                                                currency = currency.name
                                            ),
                                        );
                                    }

                                    Ok(())
                                }
                                Err(e) => {
                                    user.respond("failed to windfall :(");
                                    utils::log_err("failed to windfall", e);
                                    Ok(())
                                }
                            }
                        }),
                );
            }
            Some(..) => {
                if ctx.is_moderator() {
                    ctx.respond("Expected: boost, windfall, show, or give.");
                } else {
                    ctx.respond("Expected: give.");
                }
            }
        }

        Ok(())
    }
}
