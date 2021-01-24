use crate::irc;
use crate::irc::Sender;
use crate::player::{Event, Player};
use crate::prelude::*;
use crate::settings;
use crate::utils::{Cooldown, Duration};
use anyhow::{Context as _, Result};

/// Setup the task that sends chat feedback.
pub(crate) async fn task(
    sender: Sender,
    injector: Injector,
    chat_feedback: settings::Var<bool>,
) -> Result<()> {
    let (mut player_stream, player) = injector.stream::<Player>().await;

    let new_feedback_loop = move |new_player: Option<Player>| match new_player {
        Some(player) => Fuse::new(feedback(player, sender.clone(), chat_feedback.clone())),
        None => Default::default(),
    };

    let feedback_loop = new_feedback_loop(player);
    tokio::pin!(feedback_loop);

    loop {
        tokio::select! {
            player = player_stream.recv() => {
                feedback_loop.set(new_feedback_loop(player));
            }
            result = &mut feedback_loop => {
                if let Err(e) = result.context("feedback loop errored") {
                    return Err(e.into());
                }
            }
        }
    }
}

/// Notifications from the player.
async fn feedback(
    player: Player,
    sender: irc::Sender,
    chat_feedback: settings::Var<bool>,
) -> Result<()> {
    let mut configured_cooldown = Cooldown::from_duration(Duration::seconds(10));
    let mut rx = player.subscribe().await;

    loop {
        let e = rx.recv().await?;
        log::trace!("Player event: {:?}", e);

        match e {
            Event::Detached => {
                sender.privmsg("Player is detached!").await;
            }
            Event::Playing(feedback, item) => {
                if !feedback || !chat_feedback.load().await {
                    continue;
                }

                if let Some(item) = item {
                    let message = match item.user.as_ref() {
                        Some(user) => {
                            format!("Now playing: {}, requested by {}.", item.what(), user)
                        }
                        None => format!("Now playing: {}.", item.what(),),
                    };

                    sender.privmsg(message).await;
                } else {
                    sender.privmsg("Now playing.").await;
                }
            }
            Event::Skip => {
                sender.privmsg("Skipping song.").await;
            }
            Event::Pausing => {
                if !chat_feedback.load().await {
                    continue;
                }

                sender.privmsg("Pausing playback.").await;
            }
            Event::Empty => {
                sender
                    .privmsg("Song queue is empty (use !song request <spotify-id> to add more).")
                    .await;
            }
            Event::NotConfigured => {
                if configured_cooldown.is_open() {
                    sender.privmsg("Player has not been configured!").await;
                }
            }
            // other event we don't care about
            _ => (),
        }
    }
}
