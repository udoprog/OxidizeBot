use crate::{
    auth::Scope,
    command,
    currency::Currency,
    irc, module, player,
    player::{AddTrackError, Event, Item, PlayThemeError, Player},
    prelude::*,
    settings, track_id,
    track_id::TrackId,
    utils::{self, Cooldown, Duration},
};
use anyhow::{Context as _, Result};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;

const EXAMPLE_SEARCH: &str = "queen we will rock you";

/// Handler for the `!song` command.
pub struct Handler {
    enabled: settings::Var<bool>,
    player: injector::Var<Option<Player>>,
    request_help_cooldown: Mutex<Cooldown>,
    request_reward: settings::Var<u32>,
    currency: injector::Var<Option<Currency>>,
    spotify: Constraint,
    youtube: Constraint,
}

impl Handler {
    async fn handle_request(&self, ctx: &mut command::Context, player: Player) -> Result<()> {
        let q = ctx.rest().trim().to_string();

        if q.is_empty() {
            self.request_help(ctx, None).await;
            return Ok(());
        }

        let currency: Option<Currency> = self.currency.load().await;
        let request_reward = self.request_reward.load().await;
        let spotify = self.spotify.clone();
        let youtube = self.youtube.clone();
        let user = ctx.user.clone();

        let track_id = match TrackId::parse_with_urls(&q) {
            Ok(track_id) => Some(track_id),
            Err(e) => {
                match e {
                    // NB: fall back to searching.
                    track_id::ParseTrackIdError::MissingUriPrefix => (),
                    // show other errors.
                    e => {
                        log::warn!("bad song request: {}", e);
                        let e = format!("{} :(", e);
                        self.request_help(ctx, Some(e.as_str())).await;
                        return Ok(());
                    }
                }

                log::info!("Failed to parse as URL/URI: {}: {}", q, e);
                None
            }
        };

        let user = match user.real() {
            Some(user) => user,
            None => {
                user.respond("Only real users can request songs").await;
                return Ok(());
            }
        };

        let track_id = match track_id {
            Some(track_id) => Some(track_id),
            None => player.search_track(q.as_str()).await?,
        };

        let track_id = match track_id {
            Some(track_id) => track_id,
            None => {
                respond!(
                    user,
                    "Could not find a track matching your request, sorry :("
                );
                return Ok(());
            }
        };

        let (what, has_scope, enabled) = match track_id {
            TrackId::Spotify(..) => {
                let enabled = spotify.enabled.load().await;
                ("Spotify", user.has_scope(Scope::SongSpotify).await, enabled)
            }
            TrackId::YouTube(..) => {
                let enabled = youtube.enabled.load().await;
                ("YouTube", user.has_scope(Scope::SongYouTube).await, enabled)
            }
        };

        if !enabled {
            respond!(
                user,
                "{} song requests are currently not enabled, sorry :(",
                what
            );
            return Ok(());
        }

        if !has_scope {
            respond!(
                user,
                "You are not allowed to do {what} requests, sorry :(",
                what = what
            );
            return Ok(());
        }

        let max_duration = match track_id {
            TrackId::Spotify(_) => spotify.max_duration.load().await,
            TrackId::YouTube(_) => youtube.max_duration.load().await,
        };

        let min_currency = match track_id {
            TrackId::Spotify(_) => spotify.min_currency.load().await,
            TrackId::YouTube(_) => youtube.min_currency.load().await,
        };

        let has_bypass_constraints = user.has_scope(Scope::SongBypassConstraints).await;

        if !has_bypass_constraints {
            match min_currency {
                // don't test if min_currency is not defined.
                0 => (),
                min_currency => {
                    let currency = match currency.as_ref() {
                        Some(currency) => currency,
                        None => {
                            respond!(
                                user,
                                "No currency configured for stream, but it is required."
                            );
                            return Ok(());
                        }
                    };

                    let balance = currency
                        .balance_of(user.channel(), user.name())
                        .await?
                        .unwrap_or_default();

                    if balance.balance < min_currency {
                        respond!(
                            user,
                            "You don't have enough {currency} to request songs. Need {required}, but you have {balance}, sorry :(",
                            currency = currency.name,
                            required = min_currency,
                            balance = balance.balance,
                        );

                        return Ok(());
                    }
                }
            }
        }

        let result = player
            .add_track(user.name(), track_id, has_bypass_constraints, max_duration)
            .await;

        // AFTER HERE

        let (pos, item) = match result {
            Ok((pos, item)) => (pos, item),
            Err(AddTrackError::UnsupportedPlaybackMode) => {
                respond!(
                    user,
                    "Playback mode not supported for the given track type, sorry :("
                );

                return Ok(());
            }
            Err(AddTrackError::PlayerClosed(reason)) => {
                match reason {
                    Some(reason) => {
                        respond!(user, reason.as_str());
                    }
                    None => {
                        respond!(user, "Player is closed from further requests, sorry :(");
                    }
                }

                return Ok(());
            }
            Err(AddTrackError::QueueContainsTrack(pos)) => {
                respond!(
                    user,
                    "Player already contains that track (position #{pos}).",
                    pos = pos + 1,
                );

                return Ok(());
            }
            Err(AddTrackError::TooManyUserTracks(count)) => {
                match count {
                    0 => {
                        respond!(user, "Unfortunately you are not allowed to add tracks :(");
                    }
                    1 => {
                        respond!(
                            user,
                            "<3 your enthusiasm, but you already have a track in the queue.",
                        );
                    }
                    count => {
                        respond!(
                            user,
                            "<3 your enthusiasm, but you already have {count} tracks in the queue.",
                            count = count,
                        );
                    }
                }

                return Ok(());
            }
            Err(AddTrackError::QueueFull) => {
                respond!(user, "Player is full, try again later!");
                return Ok(());
            }
            Err(AddTrackError::Duplicate(when, who, limit)) => {
                let duration = Utc::now().signed_duration_since(when);

                let duration = match duration.to_std() {
                    Err(_) => None,
                    Ok(duration) => Some(utils::compact_duration(&duration)),
                };

                let limit = utils::compact_duration(&limit);

                let who = match who {
                    Some(ref who) if who == user.name() => String::from(" by you"),
                    Some(ref who) => format!(" by {}", who),
                    None => String::from(""),
                };

                let duration = match duration {
                    Some(duration) => format!(" {} ago", duration),
                    None => String::from(" not too long ago"),
                };

                respond!(
                    user,
                    "That song was requested{who}{duration}, \
                         you have to wait at least {limit} between duplicate requests!",
                    who = who,
                    duration = duration,
                    limit = limit,
                );

                return Ok(());
            }
            Err(AddTrackError::MissingAuth) => {
                respond!(
                    user,
                    "Cannot add the given song because the service has not been authenticated by the streamer!",
                );

                return Ok(());
            }
            Err(AddTrackError::NotPlayable) => {
                respond!(
                    user,
                    "This song is not available in the streamer's region :("
                );
    
                return Ok(());
            }
            Err(AddTrackError::Error(e)) => {
                return Err(e);
            }
        };

        let currency = match currency.as_ref() {
            Some(currency) if request_reward > 0 => currency,
            _ => {
                if let Some(pos) = pos {
                    respond!(
                        user,
                        "Added {what} at position #{pos}!",
                        what = item.what(),
                        pos = pos + 1
                    );
                } else {
                    respond!(user, "Added {what}!", what = item.what());
                }

                return Ok(());
            }
        };

        match currency
            .balance_add(user.channel(), user.name(), request_reward as i64)
            .await
        {
            Ok(()) => {
                if let Some(pos) = pos {
                    respond!(
                        user,
                        "Added {what} at position #{pos}, here's your {amount} {currency}!",
                        what = item.what(),
                        pos = pos + 1,
                        amount = request_reward,
                        currency = currency.name,
                    );
                } else {
                    respond!(
                        user,
                        "Added {what}, here's your {amount} {currency}!",
                        what = item.what(),
                        amount = request_reward,
                        currency = currency.name,
                    );
                }
            }
            Err(e) => {
                log_error!(e, "failed to reward user for song request");
            }
        };

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
                        ctx.user.streamer().name
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
                    let elapsed = utils::digital_duration(&current.elapsed());
                    let duration = utils::digital_duration(&current.duration());

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
                        let when = utils::compact_duration(&when);

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

                        match player.volume(volume).await? {
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
                self.handle_request(ctx, player).await?;
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
                        let length = utils::long_duration(&duration);
                        ctx.respond(format!("One song in queue with {} of play time.", length))
                            .await;
                    }
                    count => {
                        let length = utils::long_duration(&duration);
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
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        let currency = injector.var().await?;
        let settings = settings.scoped("song");

        let enabled = settings.var("enabled", false).await?;
        let chat_feedback = settings.var("chat-feedback", true).await?;
        let request_reward = settings.var("request-reward", 0).await?;

        let spotify = Constraint::build(&mut settings.scoped("spotify"), true, 0).await?;
        let youtube = Constraint::build(&mut settings.scoped("youtube"), false, 60).await?;

        let (mut player_stream, player) = injector.stream().await;

        let shared_player = injector::Var::new(player.clone());

        let future = {
            let sender = sender.clone();
            let shared_player = shared_player.clone();

            async move {
                let new_feedback_loop = move |new_player: Option<&Player>| match new_player {
                    Some(new_player) => Some(
                        feedback(new_player.clone(), sender.clone(), chat_feedback.clone()).boxed(),
                    ),
                    None => None,
                };

                let mut feedback_loop = new_feedback_loop(player.as_ref());

                loop {
                    futures::select! {
                        update = player_stream.select_next_some() => {
                            feedback_loop = new_feedback_loop(update.as_ref());
                            *shared_player.write().await = update;
                        }
                        result = feedback_loop.current() => {
                            if let Err(e) = result.context("feedback loop errored") {
                                return Err(e.into());
                            }
                        }
                    }
                }
            }
        };

        let help_cooldown = Cooldown::from_duration(Duration::seconds(5));

        handlers.insert(
            "song",
            Handler {
                enabled,
                request_help_cooldown: Mutex::new(help_cooldown),
                player: shared_player,
                request_reward,
                currency,
                spotify,
                youtube,
            },
        );

        futures.push(future.boxed());
        Ok(())
    }
}

/// Constraint for a single kind of track.
#[derive(Debug, Clone)]
struct Constraint {
    enabled: settings::Var<bool>,
    max_duration: settings::Var<Option<Duration>>,
    min_currency: settings::Var<i64>,
}

impl Constraint {
    async fn build(
        vars: &mut settings::Settings,
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

/// Notifications from the player.
async fn feedback(
    player: Player,
    sender: irc::Sender,
    chat_feedback: settings::Var<bool>,
) -> Result<()> {
    let mut configured_cooldown = Cooldown::from_duration(Duration::seconds(10));
    let mut rx = player.subscribe().await.fuse();

    loop {
        let e = rx.select_next_some().await?;
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
