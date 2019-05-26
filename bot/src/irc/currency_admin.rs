use crate::{command, currency, db};

/// Handler for the !admin command.
pub struct Handler<'a> {
    pub currency: currency::Currency,
    pub db: &'a db::Database,
}

impl command::Handler for Handler<'_> {
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

                let user = ctx_try!(ctx.next_str("<user>", "!currency show"));
                let balance = self.db.balance_of(ctx.user.target, user)?.unwrap_or(0);

                ctx.respond(format!(
                    "{user} has {balance} {name}.",
                    user = user,
                    balance = balance,
                    name = self.currency.name
                ));
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
                        currency = self.currency.name
                    ));
                    return Ok(());
                }

                let db = self.db.clone();
                let currency = self.currency.clone();
                let target = ctx.user.target.to_owned();
                let giver = ctx.user.as_owned_user();
                let is_streamer = ctx.user.is(ctx.streamer);

                ctx.spawn(async move {
                    let result = db
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
                        Err(db::BalanceTransferError::NoBalance) => {
                            giver.respond(format!(
                                "Not enough {currency} to transfer {amount}",
                                currency = currency.name,
                                amount = amount,
                            ));
                        }
                        Err(db::BalanceTransferError::Other(e)) => {
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
                ctx.check_moderator()?;

                let boosted_user =
                    db::user_id(ctx_try!(ctx.next_str("<user> <amount>", "!currency boost")));
                let amount: i64 = ctx_try!(ctx.next_parse("<user> <amount>", "!currency boost"));

                if !ctx.user.is(ctx.streamer) && ctx.user.is(&boosted_user) {
                    ctx.respond("You gonna have to play by the rules (or ask another mod) :(");
                    return Ok(());
                }

                let db = self.db.clone();
                let user = ctx.user.as_owned_user();
                let currency = self.currency.clone();

                ctx.spawn(async move {
                    let result = db
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
                ctx.check_moderator()?;

                let user = ctx.user.as_owned_user();
                let amount: i64 = ctx_try!(ctx.next_parse("<amount>", "!currency windfall"));
                let currency = self.currency.clone();
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
