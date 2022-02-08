use crate::auth::Scope;
use crate::command;
use crate::currency::Currency;
use crate::irc;
use crate::module;
use crate::player;
use crate::player::{AddTrackError, Item, PlayThemeError, Player};
use crate::prelude::*;
use crate::settings;
use crate::utils::{self, Cooldown, Duration};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

mod feedback;
mod redemption;
mod requester;

const EXAMPLE_SEARCH: &str = "queen we will rock you";

/// Handler for the `!song` command.
pub struct Handler {
    enabled: settings::Var<bool>,
    player: injector::Ref<Player>,
    request_help_cooldown: Mutex<Cooldown>,
    currency: injector::Ref<Currency>,
    requester: requester::SongRequester,
}

impl Handler {
    async fn handle_request(&self, ctx: &mut command::Context, player: &Player) -> Result<()> {
        let q = ctx.rest().trim().to_string();

        let currency: Option<Currency> = self.currency.load().await;

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
                &q,
                user.channel(),
                user.name(),
                Some(&user),
                requester::RequestCurrency::BotCurrency(currency.as_ref()),
                player,
            )
            .await
        {
            Ok(outcome) => {
                respond!(user, "{}", outcome)
            }
            Err(e) => match e {
                requester::RequestError::BadRequest(reason) => {
                    self.request_help(ctx, reason.as_deref()).await;
                }
                requester::RequestError::AddTrackError(e) => match e {
                    AddTrackError::Error(e) => {
                        return Err(e);
                    }
                    e => {
                        respond!(user, "{}", e);
                    }
                },
                requester::RequestError::Error(e) => {
                    return Err(e);
                }
                e => {
                    respond!(user, "{}", e);
                }
            },
        }

        Ok(())
    }

    /// Provide a help message instructing the user how to perform song requests.
    async fn request_help(&self, ctx: &mut command::Context, reason: Option<&str>) {
        if !self.request_help_cooldown.lock().await.is_open() {
            if let Some(reason) = reason {
                respond!(ctx, reason);
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

        respond!(ctx, response);
    }
}

#[async_trait]
impl command::Handler for Handler {
    fn scope(&self) -> Option<Scope> {
        Some(Scope::Song)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let player = self
            .player
            .load()
            .await
            .ok_or_else(|| respond_err!("No player configured"))?;

        match ctx.next().as_deref() {
            Some("theme") => {
                ctx.check_scope(Scope::SongTheme).await?;
                let name = ctx.next_str("<name>")?;
                let user = ctx.user.clone();

                match player.play_theme(user.channel(), name.as_str()).await {
                    Ok(()) => (),
                    Err(PlayThemeError::NoSuchTheme) => {
                        user.respond("No such theme :(").await;
                    }
                    Err(PlayThemeError::NotConfigured) => {
                        user.respond("Theme system is not configured :(").await;
                    }
                    Err(PlayThemeError::Error(e)) => {
                        user.respond("There was a problem playing that theme :(")
                            .await;
                        log_error!(e, "failed to add song");
                    }
                    Err(PlayThemeError::MissingAuth) => {
                        user.respond(
                            "Cannot play the given theme because the service has not been authenticated by the streamer!",
                        ).await;
                    }
                }
            }
            Some("promote") => {
                ctx.check_scope(Scope::SongEditQueue).await?;
                let index = ctx
                    .next()
                    .ok_or_else(|| respond_err!("Expected <number>"))?;
                let index = parse_queue_position(&index).await?;

                if let Some(item) = player.promote_song(ctx.user.name(), index).await? {
                    respond!(
                        ctx,
                        format!("Promoted song to head of queue: {}", item.what())
                    );
                } else {
                    respond!(ctx, "No such song to promote");
                }
            }
            Some("close") => {
                ctx.check_scope(Scope::SongEditQueue).await?;

                player
                    .close(match ctx.rest() {
                        "" => None,
                        other => Some(other.to_string()),
                    })
                    .await;

                respond!(ctx, "Closed player from further requests.");
            }
            Some("open") => {
                ctx.check_scope(Scope::SongEditQueue).await?;
                player.open().await;
                respond!(ctx, "Opened player for requests.");
            }
            Some("list") => {
                if let Some(api_url) = ctx.api_url() {
                    respond!(
                        ctx,
                        "You can find the queue at {}/player/{}",
                        api_url,
                        ctx.user.streamer().login
                    );
                    return Ok(());
                }

                let mut limit = 3usize;

                if let Some(n) = ctx.next() {
                    ctx.check_scope(Scope::SongListLimit).await?;

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
                    let elapsed = utils::digital_duration(current.elapsed());
                    let duration = utils::digital_duration(current.duration());

                    if let Some(name) = current.item.user.as_ref() {
                        respond!(
                            ctx,
                            "Current song: {}, requested by {} - {elapsed} / {duration} - {url}",
                            current.item.what(),
                            name,
                            elapsed = elapsed,
                            duration = duration,
                            url = current.item.track_id.url(),
                        );
                    } else {
                        respond!(
                            ctx,
                            "Current song: {} - {elapsed} / {duration} - {url}",
                            current.item.what(),
                            elapsed = elapsed,
                            duration = duration,
                            url = current.item.track_id.url(),
                        );
                    }
                }
                None => {
                    respond!(ctx, "No song :(");
                }
            },
            Some("purge") => {
                ctx.check_scope(Scope::SongEditQueue).await?;
                player.purge().await?;
                respond!(ctx, "Song queue purged.");
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
                                respond!(ctx, "Not a real user");
                                return Ok(());
                            }
                        };

                        (true, user.name())
                    }
                };

                let user = user.to_lowercase();

                let result = player
                    .find(|item| item.user.as_ref().map(|u| *u == user).unwrap_or_default())
                    .await;

                match result {
                    Some((when, ref item)) if when.as_secs() == 0 => {
                        if your {
                            respond!(ctx, "Your song is currently playing");
                        } else {
                            respond!(ctx, "{}'s song {} is currently playing", user, item.what());
                        }
                    }
                    Some((when, item)) => {
                        let when = utils::compact_duration(when);

                        if your {
                            respond!(
                                ctx,
                                format!("Your song {} will play in {}", item.what(), when)
                            );
                        } else {
                            respond!(ctx, "{}'s song {} will play in {}", user, item.what(), when);
                        }
                    }
                    None => {
                        if your {
                            respond!(ctx, "You don't have any songs in queue :(");
                        } else {
                            respond!(ctx, format!("{} doesn't have any songs in queue :(", user));
                        }
                    }
                }
            }
            Some("delete") => {
                let removed = match ctx.next().as_deref() {
                    Some("last") => match ctx.next() {
                        Some(last_user) => {
                            let last_user = last_user.to_lowercase();
                            ctx.check_scope(Scope::SongEditQueue).await?;
                            player.remove_last_by_user(&last_user).await?
                        }
                        None => {
                            ctx.check_scope(Scope::SongEditQueue).await?;
                            player.remove_last().await?
                        }
                    },
                    Some("mine") => {
                        let user = match ctx.user.real() {
                            Some(user) => user,
                            None => {
                                respond!(ctx, "Only real users can delete their own songs");
                                return Ok(());
                            }
                        };

                        player.remove_last_by_user(user.name()).await?
                    }
                    Some(n) => {
                        ctx.check_scope(Scope::SongEditQueue).await?;
                        let n = parse_queue_position(n).await?;
                        player.remove_at(n).await?
                    }
                    None => {
                        respond!(ctx, "Expected: last, last <user>, or mine");
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
                        ctx.check_scope(Scope::SongVolume).await?;

                        let (diff, argument) = match other.chars().next() {
                            Some('+') => (Some(true), &other[1..]),
                            Some('-') => (Some(false), &other[1..]),
                            _ => (None, other),
                        };

                        let argument = match str::parse::<u32>(argument) {
                            Ok(argument) => argument,
                            Err(_) => {
                                respond!(ctx, "expected whole number argument");
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
                                respond!(ctx, format!("Updated volume to {}.", volume));
                            }
                            None => {
                                respond!(ctx, "Cannot update volume");
                            }
                        }
                    }
                    // reading volume
                    None => match player.current_volume().await {
                        Some(volume) => {
                            respond!(ctx, format!("Current volume: {}.", volume));
                        }
                        None => {
                            respond!(ctx, "No active player");
                        }
                    },
                }
            }
            Some("skip") => {
                ctx.check_scope(Scope::SongPlaybackControl).await?;
                player.skip().await?;
            }
            Some("request") => {
                self.handle_request(ctx, &player).await?;
            }
            Some("toggle") => {
                ctx.check_scope(Scope::SongPlaybackControl).await?;
                player.toggle().await?;
            }
            Some("play") => {
                ctx.check_scope(Scope::SongPlaybackControl).await?;
                player.play().await?;
            }
            Some("pause") => {
                ctx.check_scope(Scope::SongPlaybackControl).await?;
                player.pause().await?;
            }
            Some("length") => {
                let (count, duration) = player.length().await;

                match count {
                    0 => ctx.respond("No songs in queue :(").await,
                    1 => {
                        let length = utils::long_duration(duration);
                        ctx.respond(format!("One song in queue with {} of play time.", length))
                            .await;
                    }
                    count => {
                        let length = utils::long_duration(duration);
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

                if ctx.user.has_scope(Scope::SongTheme).await {
                    alts.push("theme");
                } else {
                    alts.push("theme 🛇");
                }

                if ctx.user.has_scope(Scope::SongEditQueue).await {
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

                if ctx.user.has_scope(Scope::SongVolume).await {
                    alts.push("volume");
                } else {
                    alts.push("volume 🛇");
                }

                if ctx.user.has_scope(Scope::SongPlaybackControl).await {
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
                respond!(ctx, format!("Expected argument: {}.", alts.join(", ")));
            }
        }

        Ok(())
    }
}

pub struct Module;

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
            streamer_twitch,
            stream_info,
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
            streamer_twitch.clone(),
            stream_info.clone(),
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
        vars: &mut crate::Settings,
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
        Ok(0) => respond_bail!("Can't mess with the current song :("),
        Ok(n) => Ok(n.saturating_sub(1)),
        Err(_) => respond_bail!("Expected whole number argument"),
    }
}

/// Display the collection of songs.
async fn display_songs(
    user: &irc::User,
    has_more: Option<usize>,
    it: impl IntoIterator<Item = Arc<Item>>,
) {
    let mut lines = Vec::new();

    for (index, item) in it.into_iter().enumerate() {
        match item.user.as_ref() {
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
