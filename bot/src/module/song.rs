mod feedback;
mod redemption;
mod requester;

use anyhow::Result;
use async_trait::async_trait;
use chat::command;
use chat::module;
use common::display;
use common::models::Item;
use common::{Cooldown, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;

const EXAMPLE_SEARCH: &str = "queen we will rock you";

/// Handler for the `!song` command.
pub(crate) struct Handler {
    enabled: settings::Var<bool>,
    player: async_injector::Ref<player::Player>,
    request_help_cooldown: Mutex<Cooldown>,
    currency: async_injector::Ref<currency::Currency>,
    requester: requester::SongRequester,
    streamer: api::TwitchAndUser,
}

impl Handler {
    async fn handle_request(
        &self,
        ctx: &mut command::Context<'_>,
        player: &player::Player,
    ) -> Result<()> {
        let q = ctx.rest().trim().to_string();

        let currency: Option<currency::Currency> = self.currency.load().await;

        let user = match ctx.user.real() {
            Some(user) => user,
            None => {
                ctx.respond("Only real users can request songs").await;
                return Ok(());
            }
        };

        match self
            .requester
            .request(
                ctx.channel(),
                &q,
                user.login(),
                Some(&user),
                requester::RequestCurrency::BotCurrency(currency.as_ref()),
                player,
            )
            .await
        {
            Ok(outcome) => {
                chat::respond!(user, "{}", outcome)
            }
            Err(e) => match e {
                requester::RequestError::BadRequest(reason) => {
                    self.request_help(ctx, reason.as_deref()).await;
                }
                requester::RequestError::AddTrackError(e) => match e {
                    player::AddTrackError::Error(e) => {
                        return Err(e);
                    }
                    e => {
                        chat::respond!(user, "{}", e);
                    }
                },
                requester::RequestError::Error(e) => {
                    return Err(e);
                }
                e => {
                    chat::respond!(user, "{}", e);
                }
            },
        }

        Ok(())
    }

    /// Provide a help message instructing the user how to perform song requests.
    async fn request_help(&self, ctx: &mut command::Context<'_>, reason: Option<&str>) {
        if !self.request_help_cooldown.lock().await.is_open() {
            if let Some(reason) = reason {
                chat::respond!(ctx, reason);
            }

            return;
        }

        let mut response = format!(
            "You can request a song from Spotify with \
                <search>, like \"{search}\". You can also use an URI or an URL if you feel adventurous PogChamp",
            search = EXAMPLE_SEARCH,
        );

        if let Some(reason) = reason {
            response = format!("{}. {}", reason, response);
        }

        chat::respond!(ctx, response);
    }
}

#[async_trait]
impl command::Handler for Handler {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Song)
    }

    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let player = self
            .player
            .load()
            .await
            .ok_or(chat::respond_err!("No player configured"))?;

        match ctx.next().as_deref() {
            Some("theme") => {
                ctx.check_scope(auth::Scope::SongTheme).await?;
                let name = ctx.next_str("<name>")?;

                match player.play_theme(ctx.channel(), name.as_str()).await {
                    Ok(()) => (),
                    Err(player::PlayThemeError::NoSuchTheme) => {
                        ctx.user.respond("No such theme :(").await;
                    }
                    Err(player::PlayThemeError::NotConfigured) => {
                        ctx.user.respond("Theme system is not configured :(").await;
                    }
                    Err(player::PlayThemeError::Error(e)) => {
                        ctx.user
                            .respond("There was a problem playing that theme :(")
                            .await;
                        common::log_error!(e, "Failed to add song");
                    }
                    Err(player::PlayThemeError::MissingAuth) => {
                        ctx.user.respond(
                            "Cannot play the given theme because the service has not been authenticated by the streamer!",
                        ).await;
                    }
                }
            }
            Some("promote") => {
                ctx.check_scope(auth::Scope::SongEditQueue).await?;
                let index = ctx.next().ok_or(chat::respond_err!("Expected <number>"))?;
                let index = parse_queue_position(&index).await?;

                if let Some(item) = player.promote_song(ctx.user.name(), index).await? {
                    chat::respond!(
                        ctx,
                        format!("Promoted song to head of queue: {}", item.what())
                    );
                } else {
                    chat::respond!(ctx, "No such song to promote");
                }
            }
            Some("close") => {
                ctx.check_scope(auth::Scope::SongEditQueue).await?;

                player
                    .close(match ctx.rest() {
                        "" => None,
                        other => Some(other.to_string()),
                    })
                    .await;

                chat::respond!(ctx, "Closed player from further requests.");
            }
            Some("open") => {
                ctx.check_scope(auth::Scope::SongEditQueue).await?;
                player.open().await;
                chat::respond!(ctx, "Opened player for requests.");
            }
            Some("list") => {
                if let Some(api_url) = ctx.api_url() {
                    chat::respond!(
                        ctx,
                        "You can find the queue at {}/player/{}",
                        api_url,
                        self.streamer.user.login
                    );
                    return Ok(());
                }

                let mut limit = 3usize;

                if let Some(n) = ctx.next() {
                    ctx.check_scope(auth::Scope::SongListLimit).await?;

                    if let Ok(n) = str::parse(&n) {
                        limit = n;
                    }
                }

                let items = player.list().await;

                let has_more = if items.len() > limit {
                    Some(items.len() - limit)
                } else {
                    None
                };

                display_songs(&ctx.user, has_more, items.iter().take(limit).cloned()).await;
            }
            Some("current") => match player.current().await {
                Some(current) => {
                    let elapsed = display::digital_duration(current.elapsed());
                    let duration = display::digital_duration(current.item().duration());

                    if let Some(name) = current.item().user() {
                        chat::respond!(
                            ctx,
                            "Current song: {}, requested by {} - {elapsed} / {duration} - {url}",
                            current.item().what(),
                            name,
                            elapsed = elapsed,
                            duration = duration,
                            url = current.item().track_id().url(),
                        );
                    } else {
                        chat::respond!(
                            ctx,
                            "Current song: {} - {elapsed} / {duration} - {url}",
                            current.item().what(),
                            elapsed = elapsed,
                            duration = duration,
                            url = current.item().track_id().url(),
                        );
                    }
                }
                None => {
                    chat::respond!(ctx, "No song :(");
                }
            },
            Some("purge") => {
                ctx.check_scope(auth::Scope::SongEditQueue).await?;
                player.purge().await?;
                chat::respond!(ctx, "Song queue purged.");
            }
            // print when your next song will play.
            Some("when") => {
                let user = ctx.next();

                let (your, user) = match &user {
                    Some(user) => (false, user.as_str()),
                    None => {
                        let user = match ctx.user.real() {
                            Some(user) => user,
                            None => {
                                chat::respond!(ctx, "Not a real user");
                                return Ok(());
                            }
                        };

                        (true, user.login())
                    }
                };

                let user = user.to_lowercase();

                let result = player
                    .find(|item| item.user().map(|u| *u == user).unwrap_or_default())
                    .await;

                match result {
                    Some((when, ref item)) if when.as_secs() == 0 => {
                        if your {
                            chat::respond!(ctx, "Your song is currently playing");
                        } else {
                            chat::respond!(
                                ctx,
                                "{}'s song {} is currently playing",
                                user,
                                item.what()
                            );
                        }
                    }
                    Some((when, item)) => {
                        let when = display::compact_duration(when);

                        if your {
                            chat::respond!(
                                ctx,
                                format!("Your song {} will play in {}", item.what(), when)
                            );
                        } else {
                            chat::respond!(
                                ctx,
                                "{}'s song {} will play in {}",
                                user,
                                item.what(),
                                when
                            );
                        }
                    }
                    None => {
                        if your {
                            chat::respond!(ctx, "You don't have any songs in queue :(");
                        } else {
                            chat::respond!(
                                ctx,
                                format!("{} doesn't have any songs in queue :(", user)
                            );
                        }
                    }
                }
            }
            Some("delete") => {
                let removed = match ctx.next().as_deref() {
                    Some("last") => match ctx.next() {
                        Some(last_user) => {
                            let last_user = last_user.to_lowercase();
                            ctx.check_scope(auth::Scope::SongEditQueue).await?;
                            player.remove_last_by_user(&last_user).await?
                        }
                        None => {
                            ctx.check_scope(auth::Scope::SongEditQueue).await?;
                            player.remove_last().await?
                        }
                    },
                    Some("mine") => {
                        let user = match ctx.user.real() {
                            Some(user) => user,
                            None => {
                                chat::respond!(ctx, "Only real users can delete their own songs");
                                return Ok(());
                            }
                        };

                        player.remove_last_by_user(user.login()).await?
                    }
                    Some(n) => {
                        ctx.check_scope(auth::Scope::SongEditQueue).await?;
                        let n = parse_queue_position(n).await?;
                        player.remove_at(n).await?
                    }
                    None => {
                        chat::respond!(ctx, "Expected: last, last <user>, or mine");
                        return Ok(());
                    }
                };

                match removed {
                    None => ctx.respond("No song removed, sorry :(").await,
                    Some(item) => ctx.respond(format!("Removed: {}!", item.what())).await,
                }
            }
            Some("volume") => {
                match ctx.next().as_deref() {
                    // setting volume
                    Some(other) => {
                        ctx.check_scope(auth::Scope::SongVolume).await?;

                        let (diff, argument) = match other.chars().next() {
                            Some('+') => (Some(true), &other[1..]),
                            Some('-') => (Some(false), &other[1..]),
                            _ => (None, other),
                        };

                        let argument = match str::parse::<u32>(argument) {
                            Ok(argument) => argument,
                            Err(_) => {
                                chat::respond!(ctx, "expected whole number argument");
                                return Ok(());
                            }
                        };

                        let volume = match diff {
                            Some(true) => player::ModifyVolume::Increase(argument),
                            Some(false) => player::ModifyVolume::Decrease(argument),
                            None => player::ModifyVolume::Set(argument),
                        };

                        match player.volume(volume).await {
                            Some(volume) => {
                                chat::respond!(ctx, format!("Updated volume to {}.", volume));
                            }
                            None => {
                                chat::respond!(ctx, "Cannot update volume");
                            }
                        }
                    }
                    // reading volume
                    None => match player.current_volume().await {
                        Some(volume) => {
                            chat::respond!(ctx, format!("Current volume: {}.", volume));
                        }
                        None => {
                            chat::respond!(ctx, "No active player");
                        }
                    },
                }
            }
            Some("skip") => {
                ctx.check_scope(auth::Scope::SongPlaybackControl).await?;
                player.skip().await?;
            }
            Some("request") => {
                self.handle_request(ctx, &player).await?;
            }
            Some("toggle") => {
                ctx.check_scope(auth::Scope::SongPlaybackControl).await?;
                player.toggle().await?;
            }
            Some("play") => {
                ctx.check_scope(auth::Scope::SongPlaybackControl).await?;
                player.play().await?;
            }
            Some("pause") => {
                ctx.check_scope(auth::Scope::SongPlaybackControl).await?;
                player.pause().await?;
            }
            Some("length") => {
                let (count, duration) = player.length().await;

                match count {
                    0 => ctx.respond("No songs in queue :(").await,
                    1 => {
                        let length = display::long_duration(duration);
                        ctx.respond(format!("One song in queue with {} of play time.", length))
                            .await;
                    }
                    count => {
                        let length = display::long_duration(duration);
                        ctx.respond(format!(
                            "{} songs in queue with {} of play time.",
                            count, length
                        ))
                        .await;
                    }
                }
            }
            _ => {
                let mut alts = Vec::new();

                if ctx.user.has_scope(auth::Scope::SongTheme).await {
                    alts.push("theme");
                } else {
                    alts.push("theme 🛇");
                }

                if ctx.user.has_scope(auth::Scope::SongEditQueue).await {
                    alts.push("promote");
                    alts.push("close");
                    alts.push("open");
                    alts.push("purge");
                } else {
                    alts.push("promote 🛇");
                    alts.push("close 🛇");
                    alts.push("open 🛇");
                    alts.push("purge 🛇");
                }

                if ctx.user.has_scope(auth::Scope::SongVolume).await {
                    alts.push("volume");
                } else {
                    alts.push("volume 🛇");
                }

                if ctx.user.has_scope(auth::Scope::SongPlaybackControl).await {
                    alts.push("skip");
                    alts.push("toggle");
                    alts.push("play");
                    alts.push("pause");
                } else {
                    alts.push("skip 🛇");
                    alts.push("toggle 🛇");
                    alts.push("play 🛇");
                    alts.push("pause 🛇");
                }

                alts.push("list");
                alts.push("current");
                alts.push("when");
                alts.push("delete");
                alts.push("request");
                alts.push("length");
                chat::respond!(ctx, format!("Expected argument: {}.", alts.join(", ")));
            }
        }

        Ok(())
    }
}

pub(crate) struct Module;

#[async_trait]
impl module::Module for Module {
    fn ty(&self) -> &'static str {
        "song"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            futures,
            sender,
            settings,
            injector,
            streamer,
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        let currency = injector.var().await;
        let settings = settings.scoped("song");

        let enabled = settings.var("enabled", false).await?;
        let chat_feedback = settings.var("chat-feedback", true).await?;
        let request_reward = settings.var("request-reward", 0).await?;

        let spotify = Constraint::build(&mut settings.scoped("spotify"), true, 0).await?;
        let youtube = Constraint::build(&mut settings.scoped("youtube"), false, 60).await?;

        let help_cooldown = Cooldown::from_duration(Duration::seconds(5));
        let requester = requester::SongRequester::new(request_reward, spotify, youtube);

        handlers.insert(
            "song",
            Handler {
                enabled,
                request_help_cooldown: Mutex::new(help_cooldown),
                player: injector.var().await,
                currency,
                requester: requester.clone(),
                streamer: streamer.clone(),
            },
        );

        futures.push(Box::pin(feedback::task(
            sender.clone(),
            injector.clone(),
            chat_feedback,
        )));

        futures.push(Box::pin(redemption::task(
            sender.clone(),
            injector.clone(),
            settings,
            requester,
            streamer.clone(),
        )));

        Ok(())
    }
}

/// Constraint for a single kind of track.
#[derive(Debug, Clone)]
pub(crate) struct Constraint {
    enabled: settings::Var<bool>,
    max_duration: settings::Var<Option<Duration>>,
    min_currency: settings::Var<i64>,
}

impl Constraint {
    pub(crate) async fn build(
        vars: &mut settings::Settings<::auth::Scope>,
        enabled: bool,
        min_currency: i64,
    ) -> Result<Self> {
        let enabled = vars.var("enabled", enabled).await?;
        let max_duration = vars.optional("max-duration").await?;
        let min_currency = vars.var("min-currency", min_currency).await?;

        Ok(Constraint {
            enabled,
            max_duration,
            min_currency,
        })
    }
}

/// Parse a queue position.
async fn parse_queue_position(n: &str) -> Result<usize> {
    match str::parse::<usize>(n) {
        Ok(0) => chat::respond_bail!("Can't mess with the current song :("),
        Ok(n) => Ok(n.saturating_sub(1)),
        Err(_) => chat::respond_bail!("Expected whole number argument"),
    }
}

/// Display the collection of songs.
async fn display_songs(
    user: &chat::User,
    has_more: Option<usize>,
    it: impl IntoIterator<Item = Arc<Item>>,
) {
    let mut lines = Vec::new();

    for (index, item) in it.into_iter().enumerate() {
        match item.user() {
            Some(user) => {
                lines.push(format!("#{}: {} ({user})", index, item.what(), user = user));
            }
            None => {
                lines.push(format!("#{}: {}", index, item.what()));
            }
        }
    }

    if lines.is_empty() {
        user.respond("Song queue is empty.").await;
        return;
    }

    if let Some(more) = has_more {
        user.respond(format!("{} ... and {} more.", lines.join("; "), more))
            .await;
        return;
    }

    user.respond(format!("{}.", lines.join("; "))).await;
}
