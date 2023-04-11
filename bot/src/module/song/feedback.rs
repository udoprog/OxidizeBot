use std::pin::pin;

use anyhow::{Context as _, Result};
use async_fuse::Fuse;
use async_injector::Injector;

use crate::irc::Sender;
use player::Event;

/// Setup the task that sends chat feedback.
pub(crate) async fn task(
    sender: Sender,
    injector: Injector,
    chat_feedback: settings::Var<bool>,
) -> Result<()> {
    let (mut player_stream, player) = injector.stream::<player::Player>().await;

    let new_feedback_loop = move |new_player: Option<player::Player>| match new_player {
        Some(player) => Fuse::new(feedback(player, sender.clone(), chat_feedback.clone())),
        None => Default::default(),
    };

    let mut feedback_loop = pin!(new_feedback_loop(player));

    loop {
        tokio::select! {
            player = player_stream.recv() => {
                feedback_loop.set(new_feedback_loop(player));
            }
            result = &mut feedback_loop => {
                result.context("feedback loop errored")?
            }
        }
    }
}

/// Notifications from the player.
async fn feedback(
    player: player::Player,
    sender: Sender,
    chat_feedback: settings::Var<bool>,
) -> Result<()> {
    let mut rx = player.subscribe().await;

    loop {
        let e = rx.recv().await?;
        tracing::trace!("Player event: {:?}", e);

        match e {
            Event::Detached => {
                sender.privmsg("Player is detached!").await;
            }
            Event::Playing(feedback, item) => {
                if !feedback || !chat_feedback.load().await {
                    continue;
                }

                if let Some(item) = item {
                    let message = match item.user() {
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
            // other event we don't care about
            _ => (),
        }
    }
}
