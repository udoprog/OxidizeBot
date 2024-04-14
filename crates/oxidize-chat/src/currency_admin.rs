use std::sync::Arc;

use anyhow::Error;
use async_injector::Injector;
use async_trait::async_trait;
use auth::Scope;
use common::display;

use crate::command;

/// Handler for the !admin command.
pub(crate) struct Handler {
    pub(crate) currency: async_injector::Ref<currency::Currency>,
}

impl Handler {
    /// Get the name of the command for the current currency.
    pub(crate) async fn command_name(&self) -> Option<Arc<String>> {
        match self.currency.read().await.as_deref() {
            Some(c) if c.command_enabled => Some(c.name.clone()),
            _ => None,
        }
    }
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<(), Error> {
        let currency = self
            .currency
            .load()
            .await
            .ok_or(respond_err!("No currency configured"))?;

        match ctx.next().as_deref() {
            None => {
                let user = ctx
                    .user
                    .real()
                    .ok_or(respond_err!("Only real users can check their balance"))?;

                let result = currency.balance_of(ctx.channel(), user.login()).await;

                match result {
                    Ok(balance) => {
                        let balance = balance.unwrap_or_default();
                        let watch_time = display::compact_duration(balance.watch_time().as_std());

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
                        common::log_error!(e, "Failed to get balance");
                    }
                }
            }
            Some("show") => {
                ctx.check_scope(Scope::CurrencyShow).await?;
                let to_show = ctx.next_str("<user>")?;

                match currency.balance_of(ctx.channel(), to_show.as_str()).await {
                    Ok(balance) => {
                        let balance = balance.unwrap_or_default();
                        let watch_time = display::compact_duration(balance.watch_time().as_std());

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
                        common::log_error!(e, "Failed to get balance");
                    }
                }
            }
            Some("give") => {
                let taker = db::user_id(&ctx.next_str("<user> <amount>")?);
                let amount: i64 = ctx.next_parse("<user> <amount>")?;

                let user = ctx
                    .user
                    .real()
                    .ok_or(respond_err!("Only real users can give currency"))?;

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
                        ctx.channel(),
                        user.login(),
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
                    Err(currency::BalanceTransferError::NoBalance) => {
                        respond!(
                            user,
                            "Not enough {currency} to transfer {amount}",
                            currency = currency.name,
                            amount = amount,
                        );
                    }
                    Err(error) => {
                        respond!(
                            user,
                            "Failed to give {currency}, sorry :(",
                            currency = currency.name
                        );
                        common::log_error!(error, "Failed to modify currency");
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
                        "You're gonna have to play by the rules (or ask the streamer nicely) :("
                    );
                    return Ok(());
                }

                currency
                    .balance_add(ctx.channel(), &boosted_user, amount)
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

                currency.add_channel_all(ctx.channel(), amount, 0).await?;

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

pub(crate) async fn setup(injector: &Injector) -> Result<Arc<Handler>, Error> {
    let currency = injector.var::<currency::Currency>().await;
    let handler = Handler { currency };
    Ok(Arc::new(handler))
}
