use crate::{
    auth::Scope,
    command,
    currency::Currency,
    irc, module,
    player::{AddTrackError, Event, Item, PlayThemeError, Player, PlayerClient},
    prelude::*,
    settings, stream_info, track_id,
    track_id::TrackId,
    utils::{self, Cooldown, Duration},
};
use chrono::Utc;
use failure::{Error, ResultExt as _};
use parking_lot::RwLock;
use std::sync::Arc;

const EXAMPLE_SEARCH: &'static str = "queen we will rock you";

/// Handler for the `!song` command.
pub struct Handler {
    enabled: Arc<RwLock<bool>>,
    stream_info: stream_info::StreamInfo,
    player: Arc<RwLock<Option<PlayerClient>>>,
    request_help_cooldown: Cooldown,
    subscriber_only: Arc<RwLock<bool>>,
    request_reward: Arc<RwLock<u32>>,
    currency: Arc<RwLock<Option<Currency>>>,
    spotify: Constraint,
    youtube: Constraint,
}

impl Handler {
    fn handle_request(
        &mut self,
        ctx: &mut command::Context<'_, '_>,
        player: PlayerClient,
    ) -> Result<(), Error> {
        let q = ctx.rest().trim().to_string();

        if q.is_empty() {
            self.request_help(ctx, None);
            return Ok(());
        }

        let currency: Option<Currency> = self.currency.read().clone();
        let stream_info = self.stream_info.clone();
        let subscriber_only = self.subscriber_only.clone();
        let request_reward = *self.request_reward.read();
        let spotify = self.spotify.clone();
        let youtube = self.youtube.clone();
        let user = ctx.user.as_owned_user();
        let is_moderator = ctx.is_moderator();
        let has_spotify_scope = ctx.has_scope(Scope::SongSpotify);
        let has_youtube_scope = ctx.has_scope(Scope::SongYouTube);

        let track_id = match TrackId::parse_with_urls(&q) {
            Ok(track_id) => Some(track_id),
            Err(e) => {
                match e {
                    // NB: fall back to searching.
                    track_id::ParseTrackIdError::MissingUriPrefix => (),
                    // show other errors.
                    e => {
                        log::warn!("bad song request by {}: {}", ctx.user.name, e);
                        let e = format!("{} :(", e);
                        self.request_help(ctx, Some(e.as_str()));
                        return Ok(());
                    }
                }

                log::info!("Failed to parse as URL/URI: {}: {}", q, e);
                None
            }
        };

        let future = async move {
            let track_id = match track_id {
                Some(track_id) => Some(track_id),
                None => player.search_track(q).await?,
            };

            let track_id = match track_id {
                Some(track_id) => track_id,
                None => {
                    user.respond("Could not find a track matching your request, sorry :(");
                    return Ok(());
                }
            };

            let (what, has_scope, enabled) = match track_id {
                TrackId::Spotify(..) => {
                    let enabled = *spotify.enabled.read();
                    ("Spotify", has_spotify_scope, enabled)
                }
                TrackId::YouTube(..) => {
                    let enabled = *youtube.enabled.read();
                    ("YouTube", has_youtube_scope, enabled)
                }
            };

            if !enabled {
                user.respond(format!(
                    "{} song requests are currently not enabled, sorry :(",
                    what
                ));
                return Ok(());
            }

            if !has_scope {
                user.respond(format!(
                    "You are not allowed to do {what} requests, sorry :(",
                    what = what
                ));
                return Ok(());
            }

            let subscriber_only_by_track = match track_id {
                TrackId::Spotify(..) => *spotify.subscriber_only.read(),
                TrackId::YouTube(..) => *youtube.subscriber_only.read(),
            };

            let subscriber_only = subscriber_only_by_track || *subscriber_only.read();

            if subscriber_only && !is_moderator {
                if !stream_info.is_subscriber(&user.name) {
                    user.respond(format!(
                        "You must be a subscriber for {what} requests, sorry :(",
                        what = what
                    ));
                    return Ok(());
                }
            }

            let max_duration = match track_id {
                TrackId::Spotify(_) => Some(spotify.max_duration.read().clone()),
                TrackId::YouTube(_) => Some(youtube.max_duration.read().clone()),
            };

            let min_currency = match track_id {
                TrackId::Spotify(_) => Some(spotify.min_currency.read().clone() as i64),
                TrackId::YouTube(_) => Some(youtube.min_currency.read().clone() as i64),
            };

            let result = player
                .add_track(
                    currency.clone(),
                    user.target.clone(),
                    user.name.clone(),
                    track_id,
                    is_moderator,
                    max_duration,
                    min_currency,
                )
                .await;

            let (pos, item) = match result {
                Ok((pos, item)) => (pos, item),
                Err(AddTrackError::PlayerClosed(reason)) => {
                    match reason {
                        Some(reason) => {
                            user.respond(reason.as_str());
                        }
                        None => {
                            user.respond("Player is closed from further requests, sorry :(");
                        }
                    }

                    return Ok(());
                }
                Err(AddTrackError::QueueContainsTrack(pos)) => {
                    user.respond(format!(
                        "Player already contains that track (position #{pos}).",
                        pos = pos + 1,
                    ));

                    return Ok(());
                }
                Err(AddTrackError::TooManyUserTracks(count)) => {
                    match count {
                        0 => {
                            user.respond("Unfortunately you are not allowed to add tracks :(");
                        }
                        1 => {
                            user.respond(
                                "<3 your enthusiasm, but you already have a track in the queue.",
                            );
                        }
                        count => {
                            user.respond(format!(
                                "<3 your enthusiasm, but you already have {count} tracks in the queue.",
                                count = count,
                            ));
                        }
                    }

                    return Ok(());
                }
                Err(AddTrackError::QueueFull) => {
                    user.respond("Player is full, try again later!");
                    return Ok(());
                }
                Err(AddTrackError::Duplicate(when, who, limit)) => {
                    let duration = Utc::now().signed_duration_since(when);

                    let duration = match duration.to_std() {
                        Err(_) => None,
                        Ok(duration) => Some(utils::compact_duration(duration)),
                    };

                    let limit = utils::compact_duration(limit);

                    let who = match who {
                        Some(ref who) if *who == user.name => String::from(" by you"),
                        Some(ref who) => format!(" by {}", who),
                        None => String::from(""),
                    };

                    let duration = match duration {
                        Some(duration) => format!(" {} ago", duration),
                        None => String::from(" not too long ago"),
                    };

                    user.respond(format!(
                        "That song was requested{who}{duration}, \
                         you have to wait at least {limit} between duplicate requests!",
                        who = who,
                        duration = duration,
                        limit = limit,
                    ));

                    return Ok(());
                }
                Err(AddTrackError::NoCurrency) => {
                    user.respond("No currency configured for stream, but it is required.");
                    return Ok(());
                }
                Err(AddTrackError::NotEnoughCurrency { balance, required }) => {
                    let currency = match currency.as_ref() {
                        Some(currency) => currency.name.to_string(),
                        None => String::from("currency"),
                    };

                    user.respond(format!(
                        "You don't have enough {currency} to request songs. Need {required}, but you have {balance}, sorry :(",
                        currency = currency,
                        required = required,
                        balance = balance,
                    ));

                    return Ok(());
                }
                Err(AddTrackError::Error(e)) => {
                    return Err(e);
                }
            };

            let currency = match currency.as_ref() {
                Some(currency) if request_reward > 0 => currency,
                _ => {
                    user.respond(format!(
                        "Added {what} at position #{pos}!",
                        what = item.what(),
                        pos = pos + 1
                    ));

                    return Ok(());
                }
            };

            match currency
                .balance_add(
                    user.target.clone(),
                    user.name.clone(),
                    request_reward as i64,
                )
                .await
            {
                Ok(()) => {
                    user.respond(format!(
                        "Added {what} at position #{pos}, here's your {amount} {currency}!",
                        what = item.what(),
                        pos = pos + 1,
                        amount = request_reward,
                        currency = currency.name,
                    ));
                }
                Err(e) => {
                    log_err!(e, "failed to reward user for song request");
                }
            };

            Ok(())
        };

        let user = ctx.user.as_owned_user();

        let future = future.map(move |result| match result {
            Ok(()) => (),
            Err(e) => {
                user.respond("There was a problem adding your song :(");
                log_err!(e, "error when adding song");
            }
        });

        ctx.spawn(future);
        Ok(())
    }

    /// Provide a help message instructing the user how to perform song requests.
    fn request_help(&mut self, ctx: &mut command::Context<'_, '_>, reason: Option<&str>) {
        if !self.request_help_cooldown.is_open() {
            if let Some(reason) = reason {
                ctx.respond(reason);
            }

            return;
        }

        let mut response = format!(
            "You can request a song from Spotify with \
                {prefix} <search>, like \"{prefix} {search}\". You can also use an URI or an URL if you feel adventurous PogChamp",
            prefix = ctx.alias.unwrap_or("!song request"),
            search = EXAMPLE_SEARCH,
        );

        if let Some(reason) = reason {
            response = format!("{}. {}", reason, response);
        }

        ctx.respond(response);
    }
}

impl command::Handler for Handler {
    fn scope(&self) -> Option<Scope> {
        Some(Scope::Song)
    }

    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let player = match self.player.read().as_ref() {
            Some(player) => player.clone(),
            None => {
                ctx.respond("No player configured");
                return Ok(());
            }
        };

        match ctx.next() {
            Some("theme") => {
                ctx.check_moderator()?;
                let name = ctx_try!(ctx.next_str("<name>", "!song theme")).to_string();

                let player = player.clone();
                let user = ctx.user.as_owned_user();

                ctx.spawn(async move {
                    match player.play_theme(user.target.clone(), name).await {
                        Ok(()) => (),
                        Err(PlayThemeError::NoSuchTheme) => {
                            user.respond("No such theme :(");
                        }
                        Err(PlayThemeError::Error(e)) => {
                            user.respond("There was a problem adding your song :(");
                            log_err!(e, "failed to add song");
                        }
                    }
                });
            }
            Some("promote") => {
                ctx.check_moderator()?;

                let index = match ctx.next().and_then(|n| parse_queue_position(&ctx.user, n)) {
                    Some(index) => index,
                    None => return Ok(()),
                };

                if let Some(item) = player.promote_song(ctx.user.name, index) {
                    ctx.respond(format!("Promoted song to head of queue: {}", item.what()));
                } else {
                    ctx.respond("No such song to promote");
                }
            }
            Some("close") => {
                ctx.check_moderator()?;

                player.close(match ctx.rest() {
                    "" => None,
                    other => Some(other.to_string()),
                });
                ctx.respond("Closed player from further requests.");
            }
            Some("open") => {
                ctx.check_moderator()?;
                player.open();
                ctx.respond("Opened player for requests.");
            }
            Some("list") => {
                if let Some(api_url) = ctx.api_url {
                    ctx.respond(format!(
                        "You can find the queue at {}/player/{}",
                        api_url, ctx.streamer
                    ));
                    return Ok(());
                }

                let mut limit = 3usize;

                if let Some(n) = ctx.next() {
                    ctx.check_moderator()?;

                    if let Ok(n) = str::parse(n) {
                        limit = n;
                    }
                }

                let items = player.list();

                let has_more = match items.len() > limit {
                    true => Some(items.len() - limit),
                    false => None,
                };

                display_songs(&ctx.user, has_more, items.iter().take(limit).cloned());
            }
            Some("current") => match player.current() {
                Some(current) => {
                    let elapsed = utils::digital_duration(&current.elapsed());
                    let duration = utils::digital_duration(&current.duration());

                    if let Some(name) = current.item.user.as_ref() {
                        ctx.respond(format!(
                            "Current song: {}, requested by {} - {elapsed} / {duration} - {url}",
                            current.item.what(),
                            name,
                            elapsed = elapsed,
                            duration = duration,
                            url = current.item.track_id.url(),
                        ));
                    } else {
                        ctx.respond(format!(
                            "Current song: {} - {elapsed} / {duration} - {url}",
                            current.item.what(),
                            elapsed = elapsed,
                            duration = duration,
                            url = current.item.track_id.url(),
                        ));
                    }
                }
                None => {
                    ctx.respond("No song :(");
                }
            },
            Some("purge") => {
                ctx.check_moderator()?;
                player.purge()?;
                ctx.respond("Song queue purged.");
            }
            // print when your next song will play.
            Some("when") => {
                let (your, user) = match ctx.next() {
                    Some(user) => {
                        ctx.check_moderator()?;
                        (false, user)
                    }
                    None => (true, ctx.user.name),
                };

                let user = user.to_lowercase();

                match player.find(|item| item.user.as_ref().map(|u| *u == user).unwrap_or_default())
                {
                    Some((when, ref item)) if when.as_secs() == 0 => {
                        if your {
                            ctx.respond("Your song is currently playing cmonBruh");
                        } else {
                            ctx.respond(format!(
                                "{}'s song {} is currently playing",
                                user,
                                item.what()
                            ));
                        }
                    }
                    Some((when, item)) => {
                        let when = utils::compact_duration(when);

                        if your {
                            ctx.respond(format!("Your song {} will play in {}", item.what(), when));
                        } else {
                            ctx.respond(format!(
                                "{}'s song {} will play in {}",
                                user,
                                item.what(),
                                when
                            ));
                        }
                    }
                    None => {
                        if your {
                            ctx.respond("You don't have any songs in queue :(");
                        } else {
                            ctx.respond(format!("{} doesn't have any songs in queue :(", user));
                        }
                    }
                }
            }
            Some("delete") => {
                let removed = match ctx.next() {
                    Some("last") => match ctx.next() {
                        Some(last_user) => {
                            let last_user = last_user.to_lowercase();
                            ctx.check_moderator()?;
                            player.remove_last_by_user(&last_user)?
                        }
                        None => {
                            ctx.check_moderator()?;
                            player.remove_last()?
                        }
                    },
                    Some("mine") => player.remove_last_by_user(&ctx.user.name)?,
                    Some(n) => {
                        ctx.check_moderator()?;

                        let n = match parse_queue_position(&ctx.user, n) {
                            Some(n) => n,
                            None => return Ok(()),
                        };

                        player.remove_at(n)?
                    }
                    None => {
                        ctx.respond(format!("Expected: last, last <user>, or mine"));
                        return Ok(());
                    }
                };

                match removed {
                    None => ctx.respond("No song removed, sorry :("),
                    Some(item) => ctx.respond(format!("Removed: {}!", item.what())),
                }
            }
            Some("volume") => {
                match ctx.next() {
                    // setting volume
                    Some(other) => {
                        ctx.check_moderator()?;

                        let (diff, argument) = match other.chars().next() {
                            Some('+') => (Some(true), &other[1..]),
                            Some('-') => (Some(false), &other[1..]),
                            _ => (None, other),
                        };

                        let argument = match str::parse::<u32>(argument) {
                            Ok(argument) => argument,
                            Err(_) => {
                                ctx.respond("expected whole number argument");
                                return Ok(());
                            }
                        };

                        let argument = match diff {
                            Some(true) => player.current_volume().saturating_add(argument),
                            Some(false) => player.current_volume().saturating_sub(argument),
                            None => argument,
                        };

                        // clamp the volume.
                        let argument = u32::min(100, argument);
                        ctx.respond(format!("Volume set to {}.", argument));
                        player.volume(argument)?;
                    }
                    // reading volume
                    None => {
                        ctx.respond(format!("Current volume: {}.", player.current_volume()));
                    }
                }
            }
            Some("skip") => {
                ctx.check_moderator()?;
                player.skip()?;
            }
            Some("request") => {
                self.handle_request(&mut ctx, player)?;
            }
            Some("toggle") => {
                ctx.check_moderator()?;
                player.toggle()?;
            }
            Some("play") => {
                ctx.check_moderator()?;
                player.play()?;
            }
            Some("pause") => {
                ctx.check_moderator()?;
                player.pause()?;
            }
            Some("length") => {
                let (count, duration) = player.length();

                match count {
                    0 => ctx.respond("No songs in queue :("),
                    1 => {
                        let length = utils::long_duration(&duration);
                        ctx.respond(format!("One song in queue with {} of play time.", length));
                    }
                    count => {
                        let length = utils::long_duration(&duration);
                        ctx.respond(format!(
                            "{} songs in queue with {} of play time.",
                            count, length
                        ));
                    }
                }
            }
            None | Some(_) => {
                ctx.respond(format!(
                    "Expected argument to {prefix} command.",
                    prefix = ctx.alias.unwrap_or("!song"),
                ));
            }
        }

        Ok(())
    }
}

pub struct Module;

impl module::Module for Module {
    fn ty(&self) -> &'static str {
        "song"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            stream_info,
            handlers,
            futures,
            sender,
            settings,
            injector,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        let currency = injector.var(futures);
        let settings = settings.scoped("song");

        let mut vars = settings.vars();
        let enabled = vars.var("enabled", false)?;
        let chat_feedback = vars.var("chat-feedback", true)?;
        let subscriber_only = vars.var("subscriber-only", false)?;
        let request_reward = vars.var("request-reward", 0)?;
        futures.push(vars.run().boxed());

        let vars = settings.scoped("spotify");
        let mut vars = vars.vars();
        let spotify = Constraint::build(&mut vars, true, false)?;
        futures.push(vars.run().boxed());

        let vars = settings.scoped("youtube");
        let mut vars = vars.vars();
        let youtube = Constraint::build(&mut vars, false, true)?;
        futures.push(vars.run().boxed());

        let (mut player_stream, player) = injector.stream();

        let new_feedback_loop = move |player: Option<&Player>| match player {
            Some(player) => {
                Some(feedback(player.client(), sender.clone(), chat_feedback.clone()).boxed())
            }
            None => None,
        };

        let mut feedback_loop = new_feedback_loop(player.as_ref());

        let player = Arc::new(RwLock::new(player.as_ref().map(Player::client)));

        let help_cooldown = Cooldown::from_duration(Duration::seconds(5));

        handlers.insert(
            "song",
            Handler {
                enabled,
                stream_info: stream_info.clone(),
                request_help_cooldown: help_cooldown,
                player: player.clone(),
                subscriber_only,
                request_reward,
                currency,
                spotify,
                youtube,
            },
        );

        let future = async move {
            loop {
                futures::select! {
                    update = player_stream.select_next_some() => {
                        feedback_loop = new_feedback_loop(update.as_ref());
                        *player.write() = update.as_ref().map(Player::client);
                    }
                    result = feedback_loop.current() => {
                        if let Err(e) = result.context("feedback loop errored") {
                            return Err(e.into());
                        }
                    }
                }
            }
        };

        futures.push(future.boxed());
        Ok(())
    }
}

/// Constraint for a single kind of track.
#[derive(Debug, Clone)]
struct Constraint {
    enabled: Arc<RwLock<bool>>,
    max_duration: Arc<RwLock<Duration>>,
    min_currency: Arc<RwLock<i64>>,
    subscriber_only: Arc<RwLock<bool>>,
}

impl Constraint {
    fn build(
        vars: &mut settings::Vars<'_>,
        enabled: bool,
        subscriber_only: bool,
    ) -> Result<Self, Error> {
        let enabled = vars.var("enabled", enabled)?;
        let max_duration = vars.var("max-duration", Duration::seconds(60 * 10))?;
        let min_currency = vars.var("min-currency", 60)?;
        let subscriber_only = vars.var("subscriber-only", subscriber_only)?;

        Ok(Constraint {
            enabled,
            max_duration,
            min_currency,
            subscriber_only,
        })
    }
}

/// Parse a queue position.
fn parse_queue_position(user: &irc::User<'_>, n: &str) -> Option<usize> {
    match str::parse::<usize>(n) {
        Ok(0) => {
            user.respond("Can't mess with the current song :(");
            return None;
        }
        Ok(n) => Some(n.saturating_sub(1)),
        Err(_) => {
            user.respond("Expected whole number argument");
            return None;
        }
    }
}

/// Display the collection of songs.
fn display_songs(
    user: &irc::User<'_>,
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
        user.respond("Song queue is empty.");
        return;
    }

    if let Some(more) = has_more {
        user.respond(format!("{} ... and {} more.", lines.join("; "), more));
        return;
    }

    user.respond(format!("{}.", lines.join("; ")));
}

/// Notifications from the player.
async fn feedback(
    player: PlayerClient,
    sender: irc::Sender,
    chat_feedback: Arc<RwLock<bool>>,
) -> Result<(), Error> {
    let mut configured_cooldown = Cooldown::from_duration(Duration::seconds(10));
    let mut rx = player.add_rx().compat();

    while let Some(e) = rx.next().await {
        log::trace!("Player event: {:?}", e);

        match e? {
            Event::Detached => {
                sender.privmsg("Player is detached!");
            }
            Event::Playing(feedback, item) => {
                if !feedback || !*chat_feedback.read() {
                    return Ok(());
                }

                let message = match item.user.as_ref() {
                    Some(user) => format!("Now playing: {}, requested by {}.", item.what(), user),
                    None => format!("Now playing: {}.", item.what(),),
                };

                sender.privmsg(message);
            }
            Event::Pausing => {
                if !*chat_feedback.read() {
                    return Ok(());
                }

                sender.privmsg("Pausing playback.");
            }
            Event::Empty => {
                sender.privmsg(format!(
                    "Song queue is empty (use !song request <spotify-id> to add more).",
                ));
            }
            Event::NotConfigured => {
                if configured_cooldown.is_open() {
                    sender.privmsg("Player has not been configured!");
                }
            }
            // other event we don't care about
            _ => (),
        }
    }

    Ok(())
}
