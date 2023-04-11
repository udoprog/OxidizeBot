use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use async_fuse::Fuse;
use async_injector::Injector;
use common::Duration;
use tokio::time;
use tracing::Instrument;

use crate::chat::Sender;
use crate::idle;

/// Set up a reward loop.
pub(super) async fn setup(
    streamer: api::TwitchAndUser,
    sender: Sender,
    idle: idle::Idle,
    injector: Injector,
    chat_settings: settings::Settings<::auth::Scope>,
    settings: settings::Settings<::auth::Scope>,
) -> Result<impl Future<Output = Result<()>>> {
    tracing::trace!("Setting up currency loop");

    let task = Task {
        streamer,
        sender,
        idle,
        injector,
        chat_settings,
        settings,
    };

    let future = async move {
        while let Err(error) = task.run().await {
            common::log_error!(error, "Currency task errored, retrying again in 10 seconds");
            time::sleep(time::Duration::from_secs(10)).await;
        }

        Ok(())
    };

    Ok(future.in_current_span())
}

struct Task {
    streamer: api::TwitchAndUser,
    sender: Sender,
    idle: idle::Idle,
    injector: Injector,
    chat_settings: settings::Settings<::auth::Scope>,
    settings: settings::Settings<::auth::Scope>,
}

impl Task {
    async fn run(&self) -> Result<()> {
        let Task {
            streamer,
            sender,
            idle,
            injector,
            chat_settings,
            settings,
        } = &self;

        let reward = 10;
        let default_interval = Duration::seconds(60 * 10);

        let (mut interval_stream, mut reward_interval) = chat_settings
            .stream("viewer-reward/interval")
            .or_with(default_interval)
            .await?;

        let reward_percentage = chat_settings.var("viewer-reward%", 100).await?;
        let (mut viewer_reward_stream, viewer_reward) = chat_settings
            .stream("viewer-reward/enabled")
            .or_with(false)
            .await?;
        let (mut notify_rewards_stream, mut notify_rewards) = settings
            .stream("currency/notify-rewards")
            .or_with(true)
            .await?;

        let (mut ty_stream, ty) = settings.stream("currency/type").or_default().await?;
        let (mut enabled_stream, enabled) =
            settings.stream("currency/enabled").or_default().await?;
        let (mut name_stream, name) = settings.stream("currency/name").optional().await?;
        let (mut command_enabled_stream, command_enabled) = settings
            .stream("currency/command-enabled")
            .or_with(true)
            .await?;
        let (mut mysql_url_stream, mysql_url) =
            settings.stream("currency/mysql/url").optional().await?;
        let (mut mysql_schema_stream, mysql_schema) = settings
            .stream("currency/mysql/schema")
            .or_default()
            .await?;

        let (mut db_stream, db) = injector.stream::<db::Database>().await;

        let mut builder =
            currency::CurrencyBuilder::new(streamer.clone(), mysql_schema, injector.clone());

        builder.db = db;
        builder.ty = ty;
        builder.enabled = enabled;
        builder.command_enabled = command_enabled;
        builder.name = name.map(Arc::new);
        builder.mysql_url = mysql_url;

        let mut currency = builder.build_and_inject().await;

        let new_timer = |interval: &Duration, viewer_reward: bool| {
            if viewer_reward && !interval.is_empty() {
                Fuse::new(tokio::time::interval(interval.as_std()))
            } else {
                Fuse::empty()
            }
        };

        let mut timer = new_timer(&reward_interval, viewer_reward);

        loop {
            tokio::select! {
                update = interval_stream.recv() => {
                    reward_interval = update;
                    timer = new_timer(&reward_interval, viewer_reward);
                }
                update = notify_rewards_stream.recv() => {
                    notify_rewards = update;
                }
                update = db_stream.recv() => {
                    builder.db = update;
                    currency = builder.build_and_inject().await;
                }
                enabled = enabled_stream.recv() => {
                    builder.enabled = enabled;
                    currency = builder.build_and_inject().await;
                }
                update = ty_stream.recv() => {
                    builder.ty = update;
                    currency = builder.build_and_inject().await;
                }
                name = name_stream.recv() => {
                    builder.name = name.map(Arc::new);
                    currency = builder.build_and_inject().await;
                }
                mysql_url = mysql_url_stream.recv() => {
                    builder.mysql_url = mysql_url;
                    currency = builder.build_and_inject().await;
                }
                update = mysql_schema_stream.recv() => {
                    builder.mysql_schema = update;
                    currency = builder.build_and_inject().await;
                }
                command_enabled = command_enabled_stream.recv() => {
                    builder.command_enabled = command_enabled;
                    currency = builder.build_and_inject().await;
                }
                viewer_reward = viewer_reward_stream.recv() => {
                    timer = new_timer(&reward_interval, viewer_reward);
                }
                _ = timer.as_pin_mut().poll_inner(|mut i, cx| i.poll_tick(cx)) => {
                    let currency = match currency.as_ref() {
                        Some(currency) => currency,
                        None => continue,
                    };

                    let seconds = reward_interval.num_seconds() as i64;

                    tracing::trace!("Running reward loop");

                    let reward = (reward * reward_percentage.load().await as i64) / 100i64;
                    let count = currency
                        .add_channel_all(sender.channel(), reward, seconds)
                        .await?;

                    if notify_rewards && count > 0 && !idle.is_idle().await {
                        sender.privmsg(format!(
                            "/me has given {} {} to all viewers!",
                            reward, currency.name
                        )).await;
                    }
                }
            }
        }
    }
}
