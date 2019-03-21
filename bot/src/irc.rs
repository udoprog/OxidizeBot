use crate::{
    aliases, commands, config, counters,
    currency::Currency,
    db,
    features::{Feature, Features},
    oauth2, player, twitch, utils,
    utils::BoxFuture,
    words,
};
use chrono::Utc;
use failure::format_err;
use futures::{
    future::{self, Future},
    stream::Stream,
};
use hashbrown::HashSet;
use irc::{
    client::{self, ext::ClientExt, Client, IrcClient, PackedIrcClient},
    proto::{
        command::{CapSubCommand, Command},
        message::{Message, Tag},
    },
};
use setmod_notifier::{Notification, Notifier};
use std::{
    cell, fmt,
    sync::{Arc, Mutex, RwLock},
    time,
};
use tokio::timer;
use tokio_core::reactor::Core;
use tokio_threadpool::ThreadPool;

const SERVER: &'static str = "irc.chat.twitch.tv";
const TWITCH_TAGS_CAP: &'static str = "twitch.tv/tags";
const TWITCH_COMMANDS_CAP: &'static str = "twitch.tv/commands";

/// Configuration for twitch integration.
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    bot: String,
    /// Cooldown for moderator actions.
    moderator_cooldown: Option<utils::Cooldown>,
    /// Cooldown for creating clips.
    #[serde(default = "default_clip_cooldown")]
    clip_cooldown: utils::Cooldown,
    /// Name of the channel to join.
    pub channel: Arc<String>,
    /// Whether or not to notify on currency rewards.
    #[serde(default)]
    notify_rewards: bool,
}

fn default_clip_cooldown() -> utils::Cooldown {
    utils::Cooldown::from_duration(time::Duration::from_secs(15))
}

pub fn run<'a>(
    core: &mut Core,
    db: db::Database,
    twitch: twitch::Twitch,
    config: &'a config::Config,
    irc_config: &'a Config,
    token: Arc<RwLock<oauth2::Token>>,
    commands: commands::Commands<db::Database>,
    counters: counters::Counters<db::Database>,
    bad_words: words::Words<db::Database>,
    notifier: &'a Notifier,
    player: Option<&player::Player>,
) -> Result<impl Future<Item = (), Error = failure::Error> + Send + 'a, failure::Error> {
    let access_token = token
        .read()
        .expect("poisoned lock")
        .access_token()
        .to_string();

    let irc_client_config = client::data::config::Config {
        nickname: Some(irc_config.bot.clone()),
        channels: Some(vec![(*irc_config.channel).clone()]),
        password: Some(format!("oauth:{}", access_token)),
        server: Some(String::from(SERVER)),
        port: Some(6697),
        use_ssl: Some(true),
        ..client::data::config::Config::default()
    };

    let client = IrcClient::new_future(core.handle(), &irc_client_config)?;

    let PackedIrcClient(client, send_future) = core.run(client)?;
    client.identify()?;

    let sender = Sender::new(client.clone());
    sender.cap_req(TWITCH_TAGS_CAP);
    sender.cap_req(TWITCH_COMMANDS_CAP);

    let mut futures = Vec::<BoxFuture<(), failure::Error>>::new();

    if let Some(currency) = config.currency.as_ref() {
        let reward = 10;
        let interval = 60 * 10;

        let future = reward_loop(
            irc_config,
            reward,
            interval,
            db.clone(),
            twitch.clone(),
            sender.clone(),
            currency,
        );

        futures.push(Box::new(future));
    }

    let interval = 60 * 5;

    let stream_info = {
        let stream_info = Arc::new(RwLock::new(None));
        let future = stream_info_loop(config, interval, twitch.clone(), stream_info.clone());
        futures.push(Box::new(future));
        stream_info
    };

    let player = match player {
        Some(player) => {
            futures.push(Box::new(player_feedback_loop(
                irc_config,
                player,
                sender.clone(),
            )));
            Some(player.client())
        }
        None => None,
    };

    futures.push(Box::new(send_future.map_err(failure::Error::from)));

    let mut handler = MessageHandler {
        streamer: config.streamer.clone(),
        channel: irc_config.channel.clone(),
        twitch: twitch.clone(),
        db,
        sender: sender.clone(),
        moderators: HashSet::default(),
        whitelisted_hosts: &config.whitelisted_hosts,
        currency: config.currency.as_ref(),
        stream_info,
        commands,
        counters,
        bad_words,
        notifier,
        player,
        aliases: config.aliases.clone(),
        features: &config.features,
        api_url: config.api_url.as_ref(),
        thread_pool: Arc::new(ThreadPool::new()),
        moderator_cooldown: irc_config
            .moderator_cooldown
            .clone()
            .map(cell::RefCell::new),
        clip_cooldown: irc_config.clip_cooldown.clone(),
    };

    futures.push(Box::new(
        client
            .stream()
            .map_err(failure::Error::from)
            .and_then(move |m| handler.handle(&m))
            // handle any errors.
            .or_else(|e| {
                utils::log_err("failed to process message", e);
                Ok(())
            })
            .for_each(|_| Ok(())),
    ));

    Ok(future::join_all(futures).map(|_| ()))
}

/// Notifications from the player.
fn player_feedback_loop<'a>(
    config: &'a Config,
    player: &player::Player,
    sender: Sender,
) -> impl Future<Item = (), Error = failure::Error> + 'a {
    player
        .add_rx()
        .map_err(|e| format_err!("failed to receive player update: {}", e))
        .for_each({
            move |e| {
                match e {
                    player::Event::Playing(echo, _, item) => {
                        if !echo {
                            return Ok(());
                        }

                        let message = match item.user.as_ref() {
                            Some(user) => {
                                format!("Now playing: {}, requested by {}.", item.what(), user)
                            }
                            None => format!("Now playing: {}.", item.what(),),
                        };

                        sender.privmsg(config.channel.as_str(), message);
                    }
                    player::Event::Pausing => {
                        sender.privmsg(config.channel.as_str(), "Pausing playback.");
                    }
                    player::Event::Empty => {
                        sender.privmsg(
                            config.channel.as_str(),
                            format!(
                                "Song queue is empty (use !song request <spotify-id> to add more).",
                            ),
                        );
                    }
                    // other event we don't care about
                    _ => {}
                }

                Ok(())
            }
        })
}

/// Set up a reward loop.
fn reward_loop<'a>(
    config: &'a Config,
    reward: i32,
    interval: u64,
    db: db::Database,
    twitch: twitch::Twitch,
    sender: Sender,
    currency: &'a Currency,
) -> impl Future<Item = (), Error = failure::Error> + 'a {
    // Add currency timer.
    timer::Interval::new_interval(time::Duration::from_secs(interval))
        .map_err(Into::into)
        // fetch all users.
        .and_then(move |_| {
            log::trace!("running reward loop");

            twitch
                .chatters(config.channel.as_str())
                .and_then(|chatters| {
                    let mut u = HashSet::new();
                    u.extend(chatters.viewers);
                    u.extend(chatters.moderators);
                    u.extend(chatters.broadcaster);

                    if u.is_empty() {
                        Err(format_err!("no chatters to reward"))
                    } else {
                        Ok(u)
                    }
                })
        })
        // update database.
        .and_then(move |u| db.balances_increment(config.channel.as_str(), u, reward))
        .map(move |_| {
            if config.notify_rewards {
                sender.privmsg(
                    config.channel.as_str(),
                    format!("/me has given {} {} to all viewers!", reward, currency.name),
                );
            }
        })
        // handle any errors.
        .or_else(|e| {
            utils::log_err("failed to reward users", e);
            Ok(())
        })
        .for_each(|_| Ok(()))
}

/// Set up a reward loop.
fn stream_info_loop<'a>(
    config: &'a config::Config,
    interval: u64,
    twitch: twitch::Twitch,
    stream_info: Arc<RwLock<Option<StreamInfo>>>,
) -> impl Future<Item = (), Error = failure::Error> + 'a {
    // Add currency timer.
    timer::Interval::new(time::Instant::now(), time::Duration::from_secs(interval))
        .map_err(failure::Error::from)
        .map(move |_| {
            log::trace!("refreshing stream info for streamer: {}", config.streamer);
        })
        .and_then(move |_| {
            let user = twitch.user_by_login(config.streamer.as_str());
            let stream = twitch.stream_by_login(config.streamer.as_str());
            let channel = twitch.channel_by_login(config.streamer.as_str());
            user.join3(stream, channel)
        })
        .and_then(move |(user, stream, channel)| {
            let mut stream_info = stream_info.write().map_err(|_| format_err!("poisoned"))?;
            // NB: user old user information in case new one is not available yet.
            let old_user = stream_info.as_mut().and_then(|s| s.user.take());

            *stream_info = Some(StreamInfo {
                user: user.or(old_user),
                game: channel.game,
                title: channel.status,
                stream: stream,
            });

            Ok(())
        })
        // handle any errors.
        .or_else(move |e| {
            log::error!(
                "failed to refresh stream info for streamer: {}: {}",
                config.streamer,
                e
            );
            Ok(())
        })
        .for_each(|_| Ok(()))
}

#[derive(Clone)]
struct Sender {
    client: IrcClient,
    thread_pool: Arc<ThreadPool>,
    limiter: Arc<Mutex<ratelimit::Limiter>>,
}

impl Sender {
    pub fn new(client: IrcClient) -> Sender {
        let limiter = ratelimit::Builder::new().frequency(10).capacity(95).build();

        Sender {
            client,
            thread_pool: Arc::new(ThreadPool::new()),
            limiter: Arc::new(Mutex::new(limiter)),
        }
    }

    /// Send a message.
    fn send(&self, m: impl Into<Message>) {
        let client = self.client.clone();
        let m = m.into();
        let limiter = Arc::clone(&self.limiter);

        self.thread_pool.spawn(future::lazy(move || {
            limiter.lock().expect("poisoned").wait();

            if let Err(e) = client.send(m) {
                utils::log_err("failed to send message", e.into());
            }

            Ok(())
        }));
    }

    /// Send a PRIVMSG.
    pub fn privmsg(&self, target: &str, f: impl fmt::Display) {
        self.send(Command::PRIVMSG(target.to_owned(), f.to_string()))
    }

    /// Send a capability request.
    pub fn cap_req(&self, cap: &str) {
        self.send(Command::CAP(
            None,
            CapSubCommand::REQ,
            Some(String::from(cap)),
            None,
        ))
    }
}

/// Handler for incoming messages.
struct MessageHandler<'a> {
    /// Current Streamer.
    streamer: String,
    /// Currench channel.
    channel: Arc<String>,
    /// API access.
    twitch: twitch::Twitch,
    /// Database.
    db: db::Database,
    /// Queue for sending messages.
    sender: Sender,
    /// Moderators.
    moderators: HashSet<String>,
    /// Whitelisted hosts for links.
    whitelisted_hosts: &'a HashSet<String>,
    /// Currency in use.
    currency: Option<&'a Currency>,
    /// Per-channel stream_infos.
    stream_info: Arc<RwLock<Option<StreamInfo>>>,
    /// All registered commands.
    commands: commands::Commands<db::Database>,
    /// All registered counters.
    counters: counters::Counters<db::Database>,
    /// Bad words.
    bad_words: words::Words<db::Database>,
    /// For sending notifications.
    notifier: &'a Notifier,
    /// Music player.
    player: Option<player::PlayerClient>,
    /// Aliases.
    aliases: aliases::Aliases,
    /// Enabled features.
    features: &'a Features,
    /// Configured API URL.
    api_url: Option<&'a String>,
    /// Thread pool used for driving futures.
    thread_pool: Arc<ThreadPool>,
    /// Active moderator cooldown.
    moderator_cooldown: Option<cell::RefCell<utils::Cooldown>>,
    /// Active clip cooldown.
    clip_cooldown: utils::Cooldown,
}

impl<'a> MessageHandler<'a> {
    /// Run as user.
    fn as_user<'m>(&self, tags: Tags<'m>, m: &'m Message) -> Result<User<'m>, failure::Error> {
        let name = m
            .source_nickname()
            .ok_or_else(|| format_err!("expected user info"))?;

        let target = m
            .response_target()
            .ok_or_else(|| format_err!("expected user info"))?;

        Ok(User {
            tags,
            sender: self.sender.clone(),
            name,
            target,
        })
    }

    /// Test if moderator.
    fn is_moderator(&self, user: &User<'_>) -> bool {
        self.moderators.contains(user.name)
    }

    /// Check that the given user is a moderator.
    fn check_moderator(&self, user: &User) -> Result<(), failure::Error> {
        // Streamer immune to cooldown and is always a moderator.
        if user.name == self.streamer {
            return Ok(());
        }

        if !self.is_moderator(user) {
            self.sender.privmsg(
                &user.target,
                format!(
                    "Do you think this is a democracy {name}? LUL",
                    name = user.name
                ),
            );

            failure::bail!("moderator access required for action");
        }

        // Test if we have moderator cooldown in effect.
        let moderator_cooldown = match self.moderator_cooldown.as_ref() {
            Some(moderator_cooldown) => moderator_cooldown,
            None => return Ok(()),
        };

        // NB: needed since check_moderator only has immutable access.
        let mut moderator_cooldown = moderator_cooldown
            .try_borrow_mut()
            .expect("mutable access already in progress");

        if moderator_cooldown.is_open() {
            return Ok(());
        }

        self.sender.privmsg(
            &user.target,
            format!(
                "{name} -> Cooldown in effect since last moderator action.",
                name = user.name
            ),
        );

        failure::bail!("moderator action cooldown");
    }

    /// Handle the !badword command.
    fn handle_bad_word<'m>(
        &mut self,
        user: User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        match it.next() {
            Some("edit") => {
                self.check_moderator(&user)?;

                let word = it.next().ok_or_else(|| format_err!("expected word"))?;
                let why = match it.rest() {
                    "" => None,
                    other => Some(other),
                };
;
                self.bad_words.edit(word, why)?;
                user.respond("Bad word edited");
            }
            Some("delete") => {
                self.check_moderator(&user)?;

                let word = it.next().ok_or_else(|| format_err!("expected word"))?;

                if self.bad_words.delete(word)? {
                    user.respond("Bad word removed.");
                } else {
                    user.respond("Bad word did not exist.");
                }
            }
            None => {
                user.respond("!badword is a word filter, removing unwanted messages.");
            }
            Some(_) => {
                user.respond("Expected: edit, or delete.");
            }
        }

        Ok(())
    }

    /// Handle admin commands.
    fn handle_admin<'m>(
        &mut self,
        user: User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        self.check_moderator(&user)?;

        match it.next() {
            Some("refresh-mods") => {
                self.sender.privmsg(user.target, "/mods");
            }
            None | Some(..) => {
                user.respond("Expected: refresh-mods.");
            }
        }

        Ok(())
    }

    /// Handle !8ball command.
    fn handle_8ball<'m>(
        &mut self,
        user: User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        use rand::Rng as _;

        static MAGIC_8BALL_ANSWER: &[&'static str] = &[
            "It is certain.",
            "It is decidedly so.",
            "Without a doubt.",
            "Yes - definitely.",
            "You may rely on it.",
            "As I see it, yes.",
            "Most likely.",
            "Outlook good.",
            "Yes.",
            "Signs point to yes.",
            "Reply hazy, try again.",
            "Ask again later.",
            "Better not tell you now.",
            "Cannot predict now.",
            "Concentrate and ask again.",
            "Don't count on it.",
            "My reply is no.",
            "My sources say no.",
            "Outlook not so good.",
            "Very doubtful.",
        ];

        let rest = it.rest();

        if rest.trim().is_empty() {
            user.respond("Ask a question.");
            return Ok(());
        }

        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0, MAGIC_8BALL_ANSWER.len() - 1);

        if let Some(answer) = MAGIC_8BALL_ANSWER.get(index) {
            user.respond(answer);
        }

        Ok(())
    }

    /// Handle !clip command.
    fn handle_clip<'m>(
        &mut self,
        user: User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        if !self.clip_cooldown.is_open() {
            user.respond("A clip was already created recently");
            return Ok(());
        }

        let stream_info = self.stream_info.read().expect("poisoned");

        let user_id = match stream_info.as_ref().and_then(|s| s.user.as_ref()) {
            Some(user) => user.id.as_str(),
            None => {
                log::error!("No information available on the current stream");
                user.respond("Cannot clip right now, stream is not live.");
                return Ok(());
            }
        };

        let title = match it.rest().trim() {
            "" => None,
            other => Some(other.to_string()),
        };

        let user = user.as_owned_user();

        let future = self
            .twitch
            .create_clip(user_id)
            .then::<_, BoxFuture<(), failure::Error>>({
                let _twitch = self.twitch.clone();

                move |result| {
                    let result = match result {
                        Ok(Some(clip)) => {
                            user.respond(format!(
                                "Created clip at {}/{}",
                                twitch::CLIPS_URL,
                                clip.id
                            ));

                            if let Some(_title) = title {
                                log::warn!("can't update title right now :(")
                            }

                            Ok(())
                        }
                        Ok(None) => {
                            user.respond("Failed to create clip, sorry :(");
                            Err(format_err!("created clip, but API returned nothing"))
                        }
                        Err(e) => {
                            user.respond("Failed to create clip, sorry :(");
                            Err(format_err!("failed to create clip: {}", e))
                        }
                    };

                    Box::new(future::result(result))
                }
            });

        self.thread_pool.spawn(future.map_err(|e| {
            utils::log_err("error when posting clip", e);
            ()
        }));

        Ok(())
    }

    /// Handle song command.
    fn handle_song<'m>(
        &mut self,
        user: User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        let player = match self.player.as_ref() {
            Some(player) => player,
            None => {
                log::warn!("No player configured for channel :(");
                return Ok(());
            }
        };

        match it.next() {
            Some("theme") => {
                self.check_moderator(&user)?;

                let name = match it.next() {
                    Some(name) => name,
                    None => {
                        user.respond("Expected: !song theme <name>");
                        failure::bail!("bad command");
                    }
                };

                let future = player.play_theme(name).then({
                    let user = user.as_owned_user();

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

                self.thread_pool.spawn(future);
            }
            Some("promote") => {
                self.check_moderator(&user)?;

                let index = match it.next() {
                    Some(index) => parse_queue_position(&user, index)?,
                    None => failure::bail!("bad command"),
                };

                if let Some(item) = player.promote_song(&user.name, index) {
                    user.respond(format!("Promoted song to head of queue: {}", item.what()));
                } else {
                    user.respond("No such song to promote");
                }
            }
            Some("close") => {
                self.check_moderator(&user)?;

                player.close(match it.rest() {
                    "" => None,
                    other => Some(other.to_string()),
                });
            }
            Some("open") => {
                self.check_moderator(&user)?;
                player.open();
            }
            Some("list") => {
                if let Some(api_url) = self.api_url {
                    user.respond(format!(
                        "You can find the queue at {}/player/{}",
                        api_url, self.streamer
                    ));
                    return Ok(());
                }

                let mut limit = 3usize;

                if let Some(n) = it.next() {
                    self.check_moderator(&user)?;

                    if let Ok(n) = str::parse(n) {
                        limit = n;
                    }
                }

                let items = player.list(limit + 1);

                let has_more = match items.len() > limit {
                    true => Some(items.len() - limit),
                    false => None,
                };

                self.display_songs(&user, has_more, items.iter().take(limit).cloned());
            }
            Some("current") => match player.current() {
                Some(item) => {
                    if let Some(name) = item.user.as_ref() {
                        user.respond(format!(
                            "Current song: {}, requested by {} ({duration}).",
                            item.what(),
                            name,
                            duration = item.duration(),
                        ));
                    } else {
                        user.respond(format!(
                            "Current song: {} ({duration})",
                            item.what(),
                            duration = item.duration()
                        ));
                    }
                }
                None => {
                    user.respond("No song :(");
                }
            },
            Some("purge") => {
                self.check_moderator(&user)?;
                player.purge()?;
                user.respond("Song queue purged.");
            }
            Some("delete") => {
                let removed = match it.next() {
                    Some("last") => match it.next() {
                        Some(last_user) => {
                            let last_user = last_user.to_lowercase();
                            self.check_moderator(&user)?;
                            player.remove_last_by_user(&last_user)?
                        }
                        None => {
                            self.check_moderator(&user)?;
                            player.remove_last()?
                        }
                    },
                    Some("mine") => player.remove_last_by_user(&user.name)?,
                    Some(n) => {
                        self.check_moderator(&user)?;
                        let n = parse_queue_position(&user, n)?;
                        player.remove_at(n)?
                    }
                    None => {
                        user.respond(format!("Expected: last, last <user>, or mine"));
                        failure::bail!("bad command");
                    }
                };

                match removed {
                    None => user.respond("No song removed, sorry :("),
                    Some(item) => user.respond(format!("Removed: {}!", item.what())),
                }
            }
            Some("volume") => {
                match it.next() {
                    // setting volume
                    Some(other) => {
                        self.check_moderator(&user)?;

                        let (diff, argument) = match other.chars().next() {
                            Some('+') => (Some(true), &other[1..]),
                            Some('-') => (Some(false), &other[1..]),
                            _ => (None, other),
                        };

                        let argument = match str::parse::<u32>(argument) {
                            Ok(argument) => argument,
                            Err(_) => {
                                user.respond("expected whole number argument");
                                failure::bail!("bad command");
                            }
                        };

                        let argument = match diff {
                            Some(true) => player.current_volume().saturating_add(argument),
                            Some(false) => player.current_volume().saturating_sub(argument),
                            None => argument,
                        };

                        // clamp the volume.
                        let argument = u32::min(100, argument);
                        user.respond(format!("Volume set to {}.", argument));
                        player.volume(argument)?;
                    }
                    // reading volume
                    None => {
                        user.respond(format!("Current volume: {}.", player.current_volume()));
                    }
                }
            }
            Some("skip") => {
                self.check_moderator(&user)?;
                player.skip()?;
            }
            Some("request") => {
                let q = it.rest();

                if !it.next().is_some() {
                    user.respond("expected: !song request <id>|<text>");
                    failure::bail!("bad command");
                }

                let track_id_future: BoxFuture<Option<player::TrackId>, failure::Error> =
                    match player::TrackId::from_url_or_uri(q) {
                        Ok(track_id) => Box::new(future::ok(Some(track_id))),
                        Err(e) => {
                            log::info!("Failed to parse as URL/URI: {}: {}", q, e);
                            Box::new(player.search_track(q))
                        }
                    };

                let future = track_id_future
                    .and_then({
                        let user = user.as_owned_user();

                        move |track_id| match track_id {
                            None => {
                                user.respond(
                                    "Could not find a track matching your request, sorry :(",
                                );
                                return Err(failure::format_err!("bad track in request"));
                            }
                            Some(track_id) => return Ok(track_id),
                        }
                    })
                    .map_err(|e| {
                        utils::log_err("failed to add track", e);
                        ()
                    })
                    .and_then({
                        let is_moderator = self.is_moderator(&user);
                        let user = user.as_owned_user();
                        let player = player.clone();

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

                self.thread_pool.spawn(future);
            }
            Some("toggle") => {
                self.check_moderator(&user)?;
                player.toggle()?;
            }
            Some("play") => {
                self.check_moderator(&user)?;
                player.play()?;
            }
            Some("pause") => {
                self.check_moderator(&user)?;
                player.pause()?;
            }
            Some("length") => {
                let (count, seconds) = player.length();

                match count {
                    0 => user.respond("No songs in queue :("),
                    1 => {
                        let length = utils::human_time(seconds as i64);
                        user.respond(format!("One song in queue with {} of play time.", length));
                    }
                    count => {
                        let length = utils::human_time(seconds as i64);
                        user.respond(format!(
                            "{} songs in queue with {} of play time.",
                            count, length
                        ));
                    }
                }
            }
            None | Some(..) => {
                if self.is_moderator(&user) {
                    user.respond("Expected: request, skip, play, pause, toggle, delete.");
                } else {
                    user.respond("Expected: !song request <request>, !song list, !song length, or !song delete mine.");
                }
            }
        }

        return Ok(());

        /// Parse a queue position.
        fn parse_queue_position(user: &User<'_>, n: &str) -> Result<usize, failure::Error> {
            match str::parse::<usize>(n) {
                Ok(0) => {
                    user.respond("Can't remove the current song :(");
                    failure::bail!("bad command");
                }
                Ok(n) => Ok(n.saturating_sub(1)),
                Err(e) => {
                    user.respond("Expected whole number argument");
                    failure::bail!("bad whole number argument: {}", e);
                }
            }
        }
    }

    /// Handle command administration.
    fn handle_command<'m>(
        &mut self,
        user: User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        match it.next() {
            Some("list") => {
                let mut names = self
                    .commands
                    .list(user.target)
                    .into_iter()
                    .map(|c| format!("!{}", c.key.name))
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    user.respond("No custom commands.");
                } else {
                    names.sort();
                    user.respond(format!("Custom commands: {}", names.join(", ")));
                }
            }
            Some("edit") => {
                self.check_moderator(&user)?;

                let name = match it.next() {
                    Some(name) => name,
                    None => {
                        user.respond("Expected name.");
                        failure::bail!("bad command");
                    }
                };

                self.commands.edit(user.target, name, it.rest())?;
                user.respond("Edited command.");
            }
            Some("delete") => {
                self.check_moderator(&user)?;

                let name = match it.next() {
                    Some(name) => name,
                    None => {
                        user.respond("Expected name.");
                        failure::bail!("bad command");
                    }
                };

                if self.commands.delete(user.target, name)? {
                    user.respond(format!("Deleted command `{}`.", name));
                } else {
                    user.respond("No such command.");
                }
            }
            None | Some(..) => {
                user.respond("Expected: list, edit, or delete.");
            }
        }

        Ok(())
    }

    /// Handle counter administration.
    fn handle_counter<'m>(
        &mut self,
        user: User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        match it.next() {
            Some("list") => {
                let mut names = self
                    .counters
                    .list(user.target)
                    .into_iter()
                    .map(|c| format!("!{}", c.key.name))
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    user.respond("No custom counters.");
                } else {
                    names.sort();
                    user.respond(format!("Custom counters: {}", names.join(", ")));
                }
            }
            Some("edit") => {
                self.check_moderator(&user)?;

                let name = match it.next() {
                    Some(name) => name,
                    None => {
                        user.respond("Expected name.");
                        failure::bail!("bad command");
                    }
                };

                self.counters.edit(user.target, name, it.rest())?;
                user.respond("Edited command.");
            }
            Some("delete") => {
                self.check_moderator(&user)?;

                let name = match it.next() {
                    Some(name) => name,
                    None => {
                        user.respond("Expected name.");
                        failure::bail!("bad command");
                    }
                };

                if self.counters.delete(user.target, name)? {
                    user.respond(format!("Deleted command `{}`.", name));
                } else {
                    user.respond("No such command.");
                }
            }
            None | Some(..) => {
                user.respond("Expected: list, edit, or delete.");
            }
        }

        Ok(())
    }

    /// Handle the uptime command.
    fn handle_uptime(&mut self, user: User<'_>) {
        let started_at = self
            .stream_info
            .read()
            .expect("poisoned")
            .as_ref()
            .and_then(|s| s.stream.as_ref().map(|s| s.started_at.clone()));

        let now = Utc::now();

        match started_at {
            // NB: very important to check that _now_ is after started at.
            Some(ref started_at) if now > *started_at => {
                let uptime = utils::human_time((now - *started_at).num_seconds());

                user.respond(format!(
                    "Stream has been live for {uptime}.",
                    uptime = uptime
                ));
            }
            Some(_) => {
                user.respond("Stream is live, but start time is weird!");
            }
            None => {
                user.respond("Stream is not live right now, try again later!");
            }
        };
    }

    /// Handle the title command.
    fn handle_title(&mut self, user: &User) {
        let title = self
            .stream_info
            .read()
            .expect("poisoned")
            .as_ref()
            .map(|s| s.title.clone());

        match title {
            Some(title) => {
                user.respond(title);
            }
            None => {
                user.respond("Stream is not live right now, try again later!");
            }
        };
    }

    /// Handle the title update.
    fn handle_update_title(&mut self, user: User<'_>, title: &str) -> Result<(), failure::Error> {
        let channel_id = user.target.trim_start_matches('#');

        let twitch = self.twitch.clone();
        let user = user.as_owned_user();
        let title = title.to_string();

        let mut request = twitch::UpdateChannelRequest::default();
        request.channel.status = Some(title);

        self.thread_pool.spawn(
            twitch
                .update_channel(channel_id, &request)
                .and_then(move |_| {
                    user.respond("Title updated!");
                    Ok(())
                })
                .or_else(|e| {
                    utils::log_err("failed to update title", e);
                    Ok(())
                }),
        );

        Ok(())
    }

    /// Handle the game command.
    fn handle_game(&mut self, user: User<'_>) {
        let game = self
            .stream_info
            .read()
            .expect("poisoned")
            .as_ref()
            .and_then(|s| s.game.clone());

        match game {
            Some(game) => {
                user.respond(game);
            }
            None => {
                user.respond("Unfortunately I don't know the game, sorry!");
            }
        };
    }

    /// Handle the game update.
    fn handle_update_game(&mut self, user: User<'_>, game: &str) -> Result<(), failure::Error> {
        let channel_id = user.target.trim_start_matches('#');

        let twitch = self.twitch.clone();
        let user = user.as_owned_user();
        let game = game.to_string();

        let mut request = twitch::UpdateChannelRequest::default();
        request.channel.game = Some(game);

        self.thread_pool.spawn(
            twitch
                .update_channel(channel_id, &request)
                .and_then(move |_| {
                    user.respond("Game updated!");
                    Ok(())
                })
                .or_else(|e| {
                    utils::log_err("failed to update game", e);
                    Ok(())
                }),
        );

        Ok(())
    }

    /// Handle a command.
    pub fn process_command<'m>(
        &mut self,
        tags: Tags<'m>,
        command: &str,
        m: &'m Message,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        let user = self.as_user(tags, m)?;

        match command {
            "ping" => {
                user.respond("What do you want?");
                self.notifier.send(Notification::Ping)?;
            }
            "admin" => {
                self.handle_admin(user, it)?;
            }
            "8ball" if self.features.test(Feature::EightBall) => {
                self.handle_8ball(user, it)?;
            }
            "clip" if self.features.test(Feature::Clip) => {
                self.handle_clip(user, it)?;
            }
            "song" if self.features.test(Feature::Song) => {
                self.handle_song(user, it)?;
            }
            "command" if self.features.test(Feature::Command) => {
                self.handle_command(user, it)?;
            }
            "counter" if self.features.test(Feature::Counter) => {
                self.handle_counter(user, it)?;
            }
            "afterstream" if self.features.test(Feature::AfterStream) => {
                self.db.insert_afterstream(&user.name, it.rest())?;
                user.respond("Reminder added.");
            }
            "badword" if self.features.test(Feature::BadWords) => {
                self.handle_bad_word(user, it)?;
            }
            "uptime" if self.features.test(Feature::Admin) => {
                self.handle_uptime(user);
            }
            "title" if self.features.test(Feature::Admin) => {
                let rest = it.rest();

                if rest.is_empty() {
                    self.handle_title(&user);
                } else {
                    self.check_moderator(&user)?;
                    self.handle_update_title(user, rest)?;
                }
            }
            "game" if self.features.test(Feature::Admin) => {
                let rest = it.rest();

                if rest.is_empty() {
                    self.handle_game(user);
                } else {
                    self.check_moderator(&user)?;
                    self.handle_update_game(user, rest)?;
                }
            }
            other => {
                if let Some(currency) = self.currency {
                    if currency.name == other {
                        let balance = self.db.balance_of(user.name)?.unwrap_or(0);
                        user.respond(format!(
                            "You have {balance} {name}.",
                            balance = balance,
                            name = currency.name
                        ));
                    }
                }

                if self.features.test(Feature::Command) {
                    if let Some(command) = self.commands.get(user.target, other) {
                        let vars = TemplateVars {
                            name: &user.name,
                            target: &user.target,
                        };
                        let response = command.render(&vars)?;
                        self.sender.privmsg(user.target, response);
                    }
                }

                if self.features.test(Feature::Counter) {
                    if let Some(counter) = self.counters.get(user.target, other) {
                        self.counters.increment(&*counter)?;

                        let vars = CounterVars {
                            name: &user.name,
                            target: &user.target,
                            count: counter.count(),
                        };

                        let response = counter.render(&vars)?;
                        self.sender.privmsg(user.target, response);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract tags from message.
    fn tags<'local>(m: &'local Message) -> Tags<'local> {
        let mut id = None;
        let mut msg_id = None;

        if let Some(tags) = m.tags.as_ref() {
            for t in tags {
                match *t {
                    Tag(ref name, Some(ref value)) if name == "id" => {
                        id = Some(value.as_str());
                    }
                    Tag(ref name, Some(ref value)) if name == "msg-id" => {
                        msg_id = Some(value.as_str());
                    }
                    _ => {}
                }
            }
        }

        Tags { id, msg_id }
    }

    /// Delete the given message.
    fn delete_message<'local>(
        &mut self,
        source: &str,
        tags: Tags<'local>,
    ) -> Result<(), failure::Error> {
        let id = match tags.id {
            Some(id) => id,
            None => return Ok(()),
        };

        log::info!("Attempting to delete message: {}", id);

        self.sender.privmsg(source, format!("/delete {}", id));
        Ok(())
    }

    /// Test if the message should be deleted.
    fn should_be_deleted(&mut self, m: &Message, message: &str) -> bool {
        let user = m.source_nickname();

        // Moderators can say whatever they want.
        if user.map(|u| self.moderators.contains(u)).unwrap_or(false) {
            return false;
        }

        if self.features.test(Feature::BadWords) {
            if let Some(word) = self.test_bad_words(message) {
                if let (Some(why), Some(user), Some(target)) =
                    (word.why.as_ref(), user, m.response_target())
                {
                    let why = why.render_to_string(&TemplateVars {
                        name: user,
                        target: target,
                    });

                    match why {
                        Ok(why) => {
                            self.sender.privmsg(target, &why);
                        }
                        Err(e) => {
                            utils::log_err("failed to render response", e);
                        }
                    }
                }

                return true;
            }
        }

        if self.features.test(Feature::UrlWhitelist) {
            if self.has_bad_link(message) {
                return true;
            }
        }

        false
    }

    /// Test the message for bad words.
    fn test_bad_words(&self, message: &str) -> Option<Arc<words::Word>> {
        let tester = self.bad_words.tester();

        for word in utils::TrimmedWords::new(message) {
            if let Some(word) = tester.test(word) {
                return Some(word);
            }
        }

        None
    }

    /// Check if the given iterator has URLs that need to be
    fn has_bad_link(&mut self, message: &str) -> bool {
        for url in utils::Urls::new(message) {
            if let Some(host) = url.host_str() {
                if !self.whitelisted_hosts.contains(host) {
                    return true;
                }
            }
        }

        false
    }

    /// Handle the given command.
    pub fn handle<'local>(&mut self, m: &'local Message) -> Result<(), failure::Error> {
        match m.command {
            Command::PRIVMSG(ref source, ref message) => {
                let tags = Self::tags(&m);

                let mut it = utils::Words::new(message);

                // NB: needs to store locally to maintain a reference to it.
                let alias = self.aliases.lookup(it.clone());

                if let Some(alias) = alias.as_ref() {
                    it = utils::Words::new(alias.as_str());
                }

                if let Some(command) = it.next() {
                    if command.starts_with('!') {
                        let command = &command[1..];

                        if let Err(e) = self.process_command(tags.clone(), command, m, &mut it) {
                            utils::log_err("failed to process command", e);
                        }
                    }
                }

                if self.should_be_deleted(m, message) {
                    self.delete_message(source, tags)?;
                }
            }
            Command::CAP(_, CapSubCommand::ACK, _, ref what) => {
                match what.as_ref().map(|w| w.as_str()) {
                    // twitch commands capabilities have been acknowledged.
                    // do what needs to happen with them (like `/mods`).
                    Some(TWITCH_COMMANDS_CAP) => {
                        // request to get a list of moderators.
                        self.sender.privmsg(self.channel.as_str(), "/mods")
                    }
                    _ => {}
                }

                log::info!(
                    "Capability Acknowledged: {}",
                    what.as_ref().map(|w| w.as_str()).unwrap_or("*")
                );
            }
            Command::JOIN(ref channel, _, _) => {
                let user = m.source_nickname().unwrap_or("?");
                log::info!("{} joined {}", user, channel);
            }
            Command::Response(..) => {
                log::info!("Response: {}", m);
            }
            Command::PING(..) | Command::PONG(..) => {
                // ignore
            }
            Command::Raw(..) => {
                log::trace!("raw command: {:?}", m);
            }
            Command::NOTICE(_, ref message) => {
                let tags = Self::tags(&m);

                match tags.msg_id {
                    // Response to /mods request.
                    Some("room_mods") => {
                        self.moderators = parse_room_mods(message);
                        return Ok(());
                    }
                    _ => {
                        log::info!("unhandled notice: {:?}", m);
                    }
                }
            }
            _ => {
                log::info!("unhandled: {:?}", m);
            }
        }

        Ok(())
    }

    /// Display the collection of songs.
    fn display_songs(
        &mut self,
        user: &User<'_>,
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
}

#[derive(Clone)]
pub struct OwnedUser {
    tags: OwnedTags,
    sender: Sender,
    name: String,
    target: String,
}

impl OwnedUser {
    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        self.sender
            .privmsg(self.target.as_str(), format!("{} -> {}", self.name, m));
    }
}

#[derive(Clone)]
pub struct User<'m> {
    tags: Tags<'m>,
    sender: Sender,
    name: &'m str,
    target: &'m str,
}

impl<'m> User<'m> {
    /// Respond to the user with a message.
    pub fn respond(&self, m: impl fmt::Display) {
        self.sender
            .privmsg(self.target, format!("{} -> {}", self.name, m));
    }

    /// Convert into an owned user.
    pub fn as_owned_user(&self) -> OwnedUser {
        OwnedUser {
            tags: self.tags.as_owned_tags(),
            sender: self.sender.clone(),
            name: self.name.to_owned(),
            target: self.target.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct StreamInfo {
    stream: Option<twitch::Stream>,
    user: Option<twitch::User>,
    title: String,
    game: Option<String>,
}

/// Struct of tags.
#[derive(Debug, Clone)]
pub struct Tags<'m> {
    /// contents of the id tag if present.
    id: Option<&'m str>,
    /// contents of the msg-id tag if present.
    msg_id: Option<&'m str>,
}

impl<'m> Tags<'m> {
    /// Convert into an owned set of tags.
    fn as_owned_tags(&self) -> OwnedTags {
        OwnedTags {
            id: self.id.map(|id| id.to_string()),
            msg_id: self.msg_id.map(|id| id.to_owned()),
        }
    }
}

/// Struct of tags.
#[derive(Debug, Clone)]
pub struct OwnedTags {
    id: Option<String>,
    msg_id: Option<String>,
}

#[derive(Debug)]
pub enum SenderThreadItem {
    Exit,
    Send(Message),
}

#[derive(serde::Serialize)]
pub struct TemplateVars<'a> {
    name: &'a str,
    target: &'a str,
}

#[derive(serde::Serialize)]
pub struct CounterVars<'a> {
    name: &'a str,
    target: &'a str,
    count: i32,
}

/// Parse the `room_mods` message.
fn parse_room_mods(message: &str) -> HashSet<String> {
    let mut out = HashSet::default();

    if let Some(index) = message.find(":") {
        let message = &message[(index + 1)..];
        out.extend(
            message
                .split(",")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from),
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::parse_room_mods;
    use hashbrown::HashSet;

    #[test]
    fn test_parse_room_mods() {
        assert_eq!(
            vec![String::from("foo"), String::from("bar")]
                .into_iter()
                .collect::<HashSet<String>>(),
            parse_room_mods("The moderators of this channel are: foo, bar")
        );

        assert_eq!(
            vec![String::from("a")]
                .into_iter()
                .collect::<HashSet<String>>(),
            parse_room_mods("The moderators of this channel are: a")
        );

        assert_eq!(
            vec![].into_iter().collect::<HashSet<String>>(),
            parse_room_mods("The moderators of this channel are:")
        );
    }
}
