use crate::{command, irc, module, player, track_id, utils, utils::BoxFuture};
use futures::{future, Future, Stream as _};
use std::sync::Arc;

const EXAMPLE_SEARCH: &'static str = "queen we will rock you";

/// Handler for the `!song` command.
pub struct Handler {
    pub player: player::PlayerClient,
    pub request_help_cooldown: utils::Cooldown,
}

impl Handler {
    /// Provide a help message instructing the user how to perform song requests.
    fn request_help(&mut self, ctx: command::Context<'_, '_>, reason: Option<&str>) {
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
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, 'm>) -> Result<(), failure::Error> {
        match ctx.next() {
            Some("theme") => {
                ctx.check_moderator()?;

                let name = match ctx.next() {
                    Some(name) => name,
                    None => {
                        ctx.respond(format!(
                            "expected {prefix} <name> to play a theme song",
                            prefix = ctx.alias.unwrap_or("!song theme")
                        ));
                        return Ok(());
                    }
                };

                let future = self.player.play_theme(name).then({
                    let user = ctx.user.as_owned_user();

                    move |r| {
                        match r {
                            Ok(()) => {}
                            Err(player::PlayThemeError::NoSuchTheme) => {
                                user.respond("No such theme :(");
                            }
                            Err(player::PlayThemeError::Error(e)) => {
                                user.respond("There was a problem adding your song :(");
                                utils::log_err("failed to add song", e);
                            }
                        }

                        Ok(())
                    }
                });

                ctx.spawn(future);
            }
            Some("promote") => {
                ctx.check_moderator()?;

                let index = match ctx.next().and_then(|n| parse_queue_position(&ctx.user, n)) {
                    Some(index) => index,
                    None => return Ok(()),
                };

                if let Some(item) = self.player.promote_song(ctx.user.name, index) {
                    ctx.respond(format!("Promoted song to head of queue: {}", item.what()));
                } else {
                    ctx.respond("No such song to promote");
                }
            }
            Some("close") => {
                ctx.check_moderator()?;

                self.player.close(match ctx.rest() {
                    "" => None,
                    other => Some(other.to_string()),
                });
            }
            Some("open") => {
                ctx.check_moderator()?;
                self.player.open();
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

                let items = self.player.list();

                let has_more = match items.len() > limit {
                    true => Some(items.len() - limit),
                    false => None,
                };

                display_songs(&ctx.user, has_more, items.iter().take(limit).cloned());
            }
            Some("current") => match self.player.current() {
                Some(current) => {
                    let elapsed = utils::digital_duration(&current.elapsed());
                    let duration = utils::digital_duration(&current.duration());

                    if let Some(name) = current.item.user.as_ref() {
                        ctx.respond(format!(
                            "Current song: {}, requested by {} - {elapsed} / {duration}.",
                            current.item.what(),
                            name,
                            elapsed = elapsed,
                            duration = duration,
                        ));
                    } else {
                        ctx.respond(format!(
                            "Current song: {} - {elapsed} / {duration}",
                            current.item.what(),
                            elapsed = elapsed,
                            duration = duration,
                        ));
                    }
                }
                None => {
                    ctx.respond("No song :(");
                }
            },
            Some("purge") => {
                ctx.check_moderator()?;
                self.player.purge()?;
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

                match self
                    .player
                    .find(|item| item.user.as_ref().map(|u| *u == user).unwrap_or_default())
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
                            self.player.remove_last_by_user(&last_user)?
                        }
                        None => {
                            ctx.check_moderator()?;
                            self.player.remove_last()?
                        }
                    },
                    Some("mine") => self.player.remove_last_by_user(&ctx.user.name)?,
                    Some(n) => {
                        ctx.check_moderator()?;

                        let n = match parse_queue_position(&ctx.user, n) {
                            Some(n) => n,
                            None => return Ok(()),
                        };

                        self.player.remove_at(n)?
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
                            Some(true) => self.player.current_volume().saturating_add(argument),
                            Some(false) => self.player.current_volume().saturating_sub(argument),
                            None => argument,
                        };

                        // clamp the volume.
                        let argument = u32::min(100, argument);
                        ctx.respond(format!("Volume set to {}.", argument));
                        self.player.volume(argument)?;
                    }
                    // reading volume
                    None => {
                        ctx.respond(format!("Current volume: {}.", self.player.current_volume()));
                    }
                }
            }
            Some("skip") => {
                ctx.check_moderator()?;
                self.player.skip()?;
            }
            Some("request") => {
                let q = ctx.rest();

                if !ctx.next().is_some() {
                    self.request_help(ctx, None);
                    return Ok(());
                }

                let track_id_future: BoxFuture<Option<player::TrackId>, failure::Error> =
                    match player::TrackId::parse(q) {
                        Ok(track_id) => Box::new(future::ok(Some(track_id))),
                        Err(e) => {
                            match e {
                                track_id::ParseTrackIdError::BadUri(_) => (),
                                ref e if e.is_bad_host_youtube() => {
                                    self.request_help(
                                        ctx,
                                        Some("Can't request songs from YouTube, sorry :("),
                                    );
                                    return Ok(());
                                }
                                e => {
                                    log::warn!("bad song request by {}: {}", ctx.user.name, e);
                                    let e = format!("{}, sorry :(", e);
                                    self.request_help(ctx, Some(e.as_str()));
                                    return Ok(());
                                }
                            }

                            log::info!("Failed to parse as URL/URI: {}: {}", q, e);
                            Box::new(self.player.search_track(q))
                        }
                    };

                let future = track_id_future.and_then({
                    let user = ctx.user.as_owned_user();

                    move |track_id| match track_id {
                        None => {
                            user.respond("Could not find a track matching your request, sorry :(");
                            return Err(failure::format_err!("bad track in request"));
                        }
                        Some(track_id) => return Ok(track_id),
                    }
                });

                let future = future.map_err(|e| {
                    utils::log_err("failed to add track", e);
                    ()
                });

                let future = future
                    .and_then({
                        let is_moderator = ctx.is_moderator();
                        let user = ctx.user.as_owned_user();
                        let player = self.player.clone();

                        move |track_id| {
                            player.add_track(&user.name, track_id, is_moderator).then(move |result| {
                                match result {
                                    Ok((pos, item)) => {
                                        user.respond(format!(
                                            "Added {what} at position #{pos}!",
                                            what = item.what(),
                                            pos = pos + 1
                                        ));
                                    }
                                    Err(player::AddTrackError::PlayerClosed(reason)) => {
                                        match reason {
                                            Some(reason) => {
                                                user.respond(reason.as_str());
                                            },
                                            None => {
                                                user.respond("Player is closed from further requests, sorry :(");
                                            }
                                        }
                                    }
                                    Err(player::AddTrackError::QueueContainsTrack(pos)) => {
                                        user.respond(format!(
                                            "Player already contains that track (position #{pos}).",
                                            pos = pos + 1,
                                        ));
                                    }
                                    Err(player::AddTrackError::TooManyUserTracks(count)) => {
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
                                    }
                                    Err(player::AddTrackError::QueueFull) => {
                                        user.respond("Player is full, try again later!");
                                    }
                                    Err(player::AddTrackError::Error(e)) => {
                                        user.respond("There was a problem adding your song :(");
                                        utils::log_err("failed to add song", e);
                                    }
                                }

                                Ok(())
                            })
                        }
                    });

                ctx.spawn(future);
            }
            Some("toggle") => {
                ctx.check_moderator()?;
                self.player.toggle()?;
            }
            Some("play") => {
                ctx.check_moderator()?;
                self.player.play()?;
            }
            Some("pause") => {
                ctx.check_moderator()?;
                self.player.pause()?;
            }
            Some("length") => {
                let (count, duration) = self.player.length();

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

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_cooldown")]
    help_cooldown: utils::Cooldown,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            help_cooldown: default_cooldown(),
        }
    }
}

fn default_cooldown() -> utils::Cooldown {
    utils::Cooldown::from_duration(utils::Duration::seconds(5))
}

pub struct Module {
    help_cooldown: utils::Cooldown,
    player: player::PlayerClient,
}

impl Module {
    pub fn load(module: &Config, player: &player::Player) -> Result<Self, failure::Error> {
        Ok(Module {
            help_cooldown: module.help_cooldown.clone(),
            player: player.client(),
        })
    }
}

impl module::Module for Module {
    fn ty(&self) -> &'static str {
        "song"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            irc_config,
            handlers,
            futures,
            sender,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        futures.push(Box::new(player_feedback_loop(
            irc_config,
            self.player.clone(),
            sender.clone(),
        )));

        handlers.insert(
            "song",
            Handler {
                request_help_cooldown: self.help_cooldown.clone(),
                player: self.player.clone(),
            },
        );
        Ok(())
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
    it: impl IntoIterator<Item = Arc<player::Item>>,
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
fn player_feedback_loop(
    config: &irc::Config,
    player: player::PlayerClient,
    sender: irc::Sender,
) -> impl Future<Item = (), Error = failure::Error> + Send + 'static {
    player
        .add_rx()
        .map_err(|e| failure::format_err!("failed to receive player update: {}", e))
        .for_each({
            let channel = config.channel.to_string();

            move |e| {
                match e {
                    player::Event::Playing(echo, item) => {
                        if !echo {
                            return Ok(());
                        }

                        let message = match item.user.as_ref() {
                            Some(user) => {
                                format!("Now playing: {}, requested by {}.", item.what(), user)
                            }
                            None => format!("Now playing: {}.", item.what(),),
                        };

                        sender.privmsg(channel.as_str(), message);
                    }
                    player::Event::Pausing => {
                        sender.privmsg(channel.as_str(), "Pausing playback.");
                    }
                    player::Event::Empty => {
                        sender.privmsg(
                            channel.as_str(),
                            format!(
                                "Song queue is empty (use !song request <spotify-id> to add more).",
                            ),
                        );
                    }
                    player::Event::NotConfigured => {
                        sender.privmsg(channel.as_str(), "Player has not been configured yet!");
                    }
                    // other event we don't care about
                    _ => {}
                }

                Ok(())
            }
        })
}
