use crate::auth::Scope;
use crate::command;
use crate::currency::{BalanceTransferError, Currency};
use crate::db;
use crate::prelude::*;
use crate::utils;
use anyhow::Error;
use std::sync::Arc;

/// Handler for the !admin command.
pub struct Handler {
    pub currency: injector::Ref<Currency>,
}

impl Handler {
    /// Get the name of the command for the current currency.
    pub async fn command_name(&self) -> Option<Arc<String>> {
        match self.currency.read().await.as_deref() {
            Some(c) if c.command_enabled => Some(c.name.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context) -> Result<(), Error> {
        let currency = self
            .currency
            .load()
            .await
            .ok_or_else(|| respond_err!("No currency configured"))?;

        match ctx.next().as_deref() {
            None => {
                let user = match ctx.user.real() {
                    Some(user) => user,
                    None => {
                        respond!(ctx, "Only real users can check their balance");
                        return Ok(());
                    }
                };

                let result = currency.balance_of(user.channel(), user.name()).await;

                match result {
                    Ok(balance) => {
                        let balance = balance.unwrap_or_default();
                        let watch_time = utils::compact_duration(balance.watch_time().as_std());

                        respond!(
                            user,
                            "You have {balance} {name} [{watch_time}].",
                            balance = balance.balance,
                            name = currency.name,
                            watch_time = watch_time,
                        );
                    }
                    Err(e) => {
                        respond!(user, "Could not get balance, sorry :(");
                        log_error!(e, "failed to get balance");
                    }
                }
            }
            Some("show") => {
                ctx.check_scope(Scope::CurrencyShow).await?;
                let to_show = ctx.next_str("<user>")?;

                match currency.balance_of(ctx.channel(), to_show.as_str()).await {
                    Ok(balance) => {
                        let balance = balance.unwrap_or_default();
                        let watch_time = utils::compact_duration(balance.watch_time().as_std());

                        respond!(
                            ctx,
                            "{user} has {balance} {name} [{watch_time}].",
                            user = to_show,
                            balance = balance.balance,
                            name = currency.name,
                            watch_time = watch_time,
                        );
                    }
                    Err(e) => {
                        respond!(ctx, "Count not get balance, sorry :(");
                        log_error!(e, "failed to get balance");
                    }
                }
            }
            Some("give") => {
                let taker = db::user_id(&ctx.next_str("<user> <amount>")?);
                let amount: i64 = ctx.next_parse("<user> <amount>")?;

                let user = match ctx.user.real() {
                    Some(user) => user,
                    None => {
                        respond!(ctx, "Only real users can give currency");
                        return Ok(());
                    }
                };

                if ctx.user.is(&taker) {
                    respond!(ctx, "Giving to... yourself? But WHY?");
                    return Ok(());
                }

                if amount <= 0 {
                    respond!(
                        ctx,
                        "Can't give negative or zero {currency} LUL",
                        currency = currency.name
                    );
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
                        respond!(
                            user,
                            "Gave {user} {amount} {currency}!",
                            user = taker,
                            amount = amount,
                            currency = currency.name
                        );
                    }
                    Err(BalanceTransferError::NoBalance) => {
                        respond!(
                            user,
                            "Not enough {currency} to transfer {amount}",
                            currency = currency.name,
                            amount = amount,
                        );
                    }
                    Err(BalanceTransferError::Other(e)) => {
                        respond!(
                            user,
                            "Failed to give {currency}, sorry :(",
                            currency = currency.name
                        );
                        log_error!(e, "failed to modify currency");
                    }
                }
            }
            Some("boost") => {
                ctx.check_scope(Scope::CurrencyBoost).await?;

                let boosted_user = db::user_id(&ctx.next_str("<user> <amount>")?);
                let amount: i64 = ctx.next_parse("<user> <amount>")?;

                if !ctx.user.is_streamer() && ctx.user.is(&boosted_user) {
                    respond!(
                        ctx,
                        "You gonna have to play by the rules (or ask another mod) :("
                    );
                    return Ok(());
                }

                currency
                    .balance_add(ctx.user.channel(), &boosted_user, amount)
                    .await?;

                if amount >= 0 {
                    respond!(
                        ctx,
                        "Gave {user} {amount} {currency}!",
                        user = boosted_user,
                        amount = amount,
                        currency = currency.name
                    );
                } else {
                    respond!(
                        ctx,
                        "Took away {amount} {currency} from {user}!",
                        user = boosted_user,
                        amount = -amount,
                        currency = currency.name
                    );
                }
            }
            Some("windfall") => {
                ctx.check_scope(Scope::CurrencyWindfall).await?;

                let amount: i64 = ctx.next_parse("<amount>")?;

                currency
                    .add_channel_all(ctx.user.channel(), amount, 0)
                    .await?;

                if amount >= 0 {
                    ctx.privmsg(format!(
                        "/me gave {amount} {currency} to EVERYONE!",
                        amount = amount,
                        currency = currency.name
                    ))
                    .await;
                } else {
                    ctx.privmsg(format!(
                        "/me took away {amount} {currency} from EVERYONE!",
                        amount = amount,
                        currency = currency.name
                    ))
                    .await;
                }
            }
            Some(..) => {
                let mut alts = Vec::new();

                alts.push("give");

                if ctx.user.has_scope(Scope::CurrencyBoost).await {
                    alts.push("boost");
                } else {
                    alts.push("boost 🛇");
                }

                if ctx.user.has_scope(Scope::CurrencyWindfall).await {
                    alts.push("windfall");
                } else {
                    alts.push("windfall 🛇");
                }

                if ctx.user.has_scope(Scope::CurrencyShow).await {
                    alts.push("show");
                } else {
                    alts.push("show 🛇");
                }

                respond!(ctx, "Expected: {alts}", alts = alts.join(", "));
            }
        }

        Ok(())
    }
}

pub async fn setup(injector: &Injector) -> Result<Arc<Handler>, Error> {
    let currency = injector.var::<Currency>().await;
    let handler = Handler { currency };
    Ok(Arc::new(handler))
}
