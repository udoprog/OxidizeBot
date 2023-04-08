use crate::api;
use crate::api::twitch::pubsub;
use crate::irc;
use crate::module::song::requester::{RequestCurrency, SongRequester};
use crate::player::Player;
use crate::prelude::*;
use anyhow::Result;

/// Task used to react to redemptions as song requests.
pub(crate) async fn task(
    sender: irc::Sender,
    injector: Injector,
    settings: crate::Settings,
    requester: SongRequester,
    streamer: api::TwitchAndUser,
) -> Result<()> {
    let (mut pubsub_stream, pubsub) = injector.stream::<pubsub::TwitchPubSub>().await;
    let (mut player_stream, player) = injector.stream::<Player>().await;
    let (mut request_redemption_stream, request_redemption) = settings
        .stream::<String>("request-redemption")
        .optional()
        .await?;

    let mut state = State {
        requester,
        streamer,
        player,
        pubsub,
        sender,
        request_redemption: request_redemption.map(Into::into),
        redemptions_stream: Fuse::empty(),
    };

    state.build();

    loop {
        tokio::select! {
            request_redemption = request_redemption_stream.recv() => {
                state.request_redemption = request_redemption.map(Into::into);
                state.build();
            }
            pubsub = pubsub_stream.recv() => {
                state.pubsub = pubsub;
                state.build();
            }
            player = player_stream.recv() => {
                state.player = player;
                state.build();
            }
            Some(redemption) = state.redemptions_stream.next() => {
                tracing::info!("Got redemption");
                state.process_redemption(redemption).await;
            }
        }
    }
}

struct State {
    requester: SongRequester,
    streamer: api::TwitchAndUser,
    pubsub: Option<pubsub::TwitchPubSub>,
    player: Option<Player>,
    sender: irc::Sender,
    request_redemption: Option<Arc<str>>,
    redemptions_stream: Fuse<pubsub::TwitchStream<pubsub::Redemption>>,
}

impl State {
    fn build(&mut self) {
        // Whether any redemptions are enabled or not.
        let any_redemptions = self.request_redemption.is_some();

        let pubsub = match (self.pubsub.as_ref(), any_redemptions) {
            (Some(pubsub), true) => pubsub,
            _ => {
                self.redemptions_stream.clear();
                return;
            }
        };

        self.redemptions_stream.set(pubsub.redemptions());
    }

    /// Process a single incoming redemption.
    async fn process_redemption(&mut self, redemption: pubsub::Redemption) {
        match &self.request_redemption {
            Some(title) if title.as_ref() == redemption.reward.title => {
                let title = title.clone();
                self.request_redemption(title.as_ref(), redemption).await;
            }
            _ => (),
        }
    }

    /// Process song request redemptions.
    async fn request_redemption(&mut self, title: &str, redemption: pubsub::Redemption) {
        let input = match redemption.user_input.as_ref() {
            Some(input) => input,
            None => {
                tracing::warn!(
                    "Got matching redemption `{}`, but it had no user input",
                    title
                );
                return;
            }
        };

        let player = match &self.player {
            Some(player) => player,
            None => {
                tracing::warn!(
                    "Got matching redemption `{}`, but player not configured",
                    title
                );
                return;
            }
        };

        tracing::trace!("Process request: {}", input);

        let result = self
            .requester
            .request(
                input,
                &redemption.user.login,
                None,
                RequestCurrency::Redemption,
                player,
            )
            .await;

        let display_name = &redemption.user.display_name;

        let status = match result {
            Ok(outcome) => {
                self.sender
                    .privmsg(utils::respond(display_name, outcome))
                    .await;

                pubsub::Status::Fulfilled
            }
            Err(e) => {
                self.sender.privmsg(utils::respond(display_name, e)).await;
                pubsub::Status::Canceled
            }
        };

        let result = self
            .streamer
            .client
            .new_update_redemption_status(&self.streamer.user.id, &redemption, status)
            .await;

        if let Err(e) = result {
            log_error!(
                e,
                "failed to update status of reward `{}`",
                redemption.reward.id
            );
        }
    }
}
