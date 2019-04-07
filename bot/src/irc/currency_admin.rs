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
                                        amount = amount,
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
                ctx.respond("Expected: boost, windfall.");
            }
        }

        Ok(())
    }
}
