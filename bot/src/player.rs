use crate::{
    api, bus, config,
    currency::Currency,
    db,
    prelude::*,
    settings,
    song_file::{SongFile, SongFileBuilder},
    spotify_id::SpotifyId,
    template::Template,
    timer,
    track_id::TrackId,
    utils,
};

use chrono::{DateTime, Utc};
use failure::{bail, format_err, Error};
use parking_lot::RwLock;
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_bus::{Bus, BusReader};
use tokio_threadpool::ThreadPool;

mod connect;
mod youtube;

static DEFAULT_CURRENT_SONG_TEMPLATE: &'static str = "Song: {{name}}{{#if artists}} by {{artists}}{{/if}}{{#if paused}} (Paused){{/if}} ({{duration}})\n{{#if user~}}Request by: @{{user~}}{{/if}}";
static DEFAULT_CURRENT_SONG_STOPPED_TEMPLATE: &'static str = "Not Playing";

/// Event used by player integrations.
#[derive(Debug)]
pub enum IntegrationEvent {
    /// Indicate that the current device changed.
    DeviceChanged,
}

/// The source of action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// Event was generated automatically, don't broadcast feedback.
    Automatic,
    /// Event was generated from user input. Broadcast feedback.
    Manual,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    /// The max queue length of the player.
    #[serde(default)]
    max_queue_length: Option<u32>,
    /// The max number of songs per user.
    #[serde(default)]
    max_songs_per_user: Option<u32>,
    /// Playlist to fall back on. Will otherwise use the saved songs of the user.
    #[serde(default)]
    playlist: Option<String>,
    /// Volume of player.
    #[serde(default)]
    volume: Option<u32>,
    /// Whether or not to echo current song.
    #[serde(default)]
    echo_current_song: Option<bool>,
    /// Device to use with connect player.
    #[serde(default)]
    device: Option<String>,
    /// Interval at which to try to sync the remote player with the local state.
    #[serde(default)]
    sync_player_interval: utils::Duration,
}

/// Information on a single track.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum Track {
    #[serde(rename = "spotify")]
    Spotify { track: api::spotify::FullTrack },
    #[serde(rename = "youtube")]
    YouTube { video: api::youtube::Video },
}

impl Track {
    /// Get artists involved as a string.
    pub fn artists(&self) -> Option<String> {
        match *self {
            Track::Spotify { ref track } => utils::human_artists(&track.artists),
            Track::YouTube { ref video } => {
                video.snippet.as_ref().and_then(|s| s.channel_title.clone())
            }
        }
    }

    /// Get name of the track.
    pub fn name(&self) -> String {
        match *self {
            Track::Spotify { ref track } => track.name.to_string(),
            Track::YouTube { ref video } => video
                .snippet
                .as_ref()
                .map(|s| s.title.as_str())
                .unwrap_or("no name")
                .to_string(),
        }
    }

    /// Convert into JSON.
    /// TODO: this is a hack to avoid breaking web API.
    pub fn to_json(&self) -> Result<serde_json::Value, Error> {
        let json = match *self {
            Track::Spotify { ref track } => serde_json::to_value(&track)?,
            Track::YouTube { ref video } => serde_json::to_value(&video)?,
        };

        Ok(json)
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub track_id: TrackId,
    pub track: Track,
    pub user: Option<String>,
    pub duration: Duration,
}

impl Item {
    /// Human readable version of playback item.
    pub fn what(&self) -> String {
        match self.track {
            Track::Spotify { ref track } => {
                if let Some(artists) = utils::human_artists(&track.artists) {
                    format!("\"{}\" by {}", track.name, artists)
                } else {
                    format!("\"{}\"", track.name)
                }
            }
            Track::YouTube { ref video } => match video.snippet.as_ref() {
                Some(snippet) => match snippet.channel_title.as_ref() {
                    Some(channel_title) => {
                        format!("\"{}\" from \"{}\"", snippet.title, channel_title)
                    }
                    None => format!("\"{}\"", snippet.title),
                },
                None => String::from("*Some YouTube Video*"),
            },
        }
    }
}

#[derive(Debug)]
pub enum Command {
    /// Skip the current song.
    Skip(Source),
    /// Toggle playback.
    Toggle(Source),
    /// Pause playback.
    Pause(Source),
    /// Start playback.
    Play(Source),
    /// Start playback on a specific song state.
    Sync { song: Option<Song> },
    /// The queue was modified.
    Modified(Source),
    /// Set the gain of the player.
    Volume(Source, u32),
    /// Play the given item as a theme at the given offset.
    Inject(Source, Arc<Item>, Duration),
}

impl Command {
    /// Get the source of a command.
    pub fn source(&self) -> Source {
        use self::Command::*;

        match *self {
            Skip(source) | Toggle(source) | Pause(source) | Play(source) | Modified(source)
            | Volume(source, ..) | Inject(source, ..) => source,
            Sync { .. } => Source::Automatic,
        }
    }
}

/// Run the player.
pub fn run(
    db: db::Database,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    config: Arc<config::Config>,
    global_bus: Arc<bus::Bus<bus::Global>>,
    youtube_bus: Arc<bus::Bus<bus::YouTube>>,
    settings: settings::Settings,
    themes: db::Themes,
) -> Result<(Player, impl Future<Output = Result<(), Error>>), Error> {
    let settings = settings.scoped("player");

    let mut futures = utils::Futures::default();

    let (connect_player, device) =
        connect::setup(&mut futures, spotify.clone(), settings.scoped("spotify"))?;
    let youtube_player = youtube::setup(
        &mut futures,
        youtube_bus.clone(),
        settings.scoped("youtube"),
    )?;

    let bus = Arc::new(RwLock::new(Bus::new(1024)));
    let queue = Queue::new(db.clone());

    let volume = Arc::new(RwLock::new(u32::min(
        100u32,
        config.player.volume.unwrap_or(50u32),
    )));

    let song = Arc::new(RwLock::new(None));
    let closed = Arc::new(RwLock::new(None));

    let (song_update_interval_stream, song_update_interval) = settings
        .stream("song-update-interval")
        .or_with(utils::Duration::seconds(1))?;

    let song_update_interval = match song_update_interval.is_empty() {
        true => None,
        false => Some(timer::Interval::new_interval(song_update_interval.as_std())),
    };

    if !config.player.sync_player_interval.is_empty() {
        log::warn!("### DEPRECATION WARNING");
        log::warn!("[player] sync_player_interval - configuration has been deprecated since it was too unreliable.");
    }

    if config.current_song.path.is_some() {
        log::warn!("`[current_song] path` configuration is deprecated.");
    }

    if config.current_song.template.is_some() {
        log::warn!("`[current_song] template` configuration is deprecated.");
    }

    if config.current_song.not_playing.is_some() {
        log::warn!("`[current_song] not_playing` configuration is deprecated.");
    }

    if !config.current_song.update_interval.is_empty() {
        log::warn!("`[current_song] update_interval` configuration is deprecated.");
    }

    let (commands_tx, commands) = mpsc::unbounded();

    let (detached_stream, detached) = settings.stream("detached").or_default()?;
    let mut vars = settings.vars();

    let duplicate_duration = vars.var("duplicate-duration", utils::Duration::default())?;

    let song_switch_feedback = vars.var(
        "song-switch-feedback",
        match config.player.echo_current_song.clone() {
            Some(value) => {
                log::warn!("`[player] echo_current_song` configuration is deprecated");
                value
            }
            None => true,
        },
    )?;

    let max_songs_per_user = vars.var(
        "max-songs-per-user",
        match config.player.max_songs_per_user.clone() {
            Some(value) => {
                log::warn!("`[player] max_songs_per_user` configuration is deprecated");
                value
            }
            None => 2,
        },
    )?;

    let max_queue_length = vars.var(
        "max-queue-length",
        match config.player.max_queue_length.clone() {
            Some(value) => {
                log::warn!("`[player] max_queue_length` configuration is deprecated");
                value
            }
            None => 30,
        },
    )?;

    futures.push(vars.run().boxed());

    let parent_player = Player {
        device: device.clone(),
        queue: queue.clone(),
        db: db.clone(),
        max_queue_length,
        max_songs_per_user,
        duplicate_duration,
        spotify: spotify.clone(),
        youtube: youtube.clone(),
        commands_tx,
        bus: bus.clone(),
        volume: volume.clone(),
        song: song.clone(),
        themes: themes.clone(),
        closed: closed.clone(),
    };

    let player = parent_player.client();

    // future to initialize the player future.
    // Yeah, I know....
    let future = async move {
        log::trace!("Waiting for token to become ready");

        // NB: since we do some work when we initialize the player, wait until
        // Spotify and YouTube are at least initialy authenticated.
        future::try_join(
            spotify.token.wait_until_ready(),
            youtube.token.wait_until_ready(),
        )
        .await?;

        // Add tracks from database.
        for song in db.list()? {
            let item = convert_item(
                spotify.clone(),
                youtube.clone(),
                song.user.clone(),
                song.track_id,
                None,
            )
            .await?;

            queue.push_back_queue(Arc::new(item));
        }

        let mixer = Mixer {
            queue,
            sidelined: Default::default(),
            fallback_items: Default::default(),
            fallback_queue: Default::default(),
        };

        let future = PlaybackFuture {
            config: config.clone(),
            spotify: spotify.clone(),
            connect_player,
            youtube_player,
            commands,
            bus: EventBus { bus },
            mixer,
            state: State::None,
            player: PlayerKind::None,
            detached,
            detached_stream,
            volume: Arc::clone(&volume),
            song: song.clone(),
            song_file: None,
            song_switch_feedback,
            song_update_interval,
            song_update_interval_stream,
            global_bus,
            timeout: None,
        };

        let p = match spotify.me_player().await? {
            Some(p) => p,
            None => return future.run(settings).await,
        };

        match Song::from_playback(&p) {
            Some(song) => {
                player.play_sync(Some(song))?;
                *volume.write() = p.device.volume_percent;
                *device.device.write() = Some(p.device);
            }
            None => {
                let devices = spotify.my_player_devices().await?;

                *device.device.write() = match config.player.device.as_ref() {
                    Some(device) => devices.into_iter().find(|d| d.name == *device),
                    None => devices.into_iter().next(),
                };

                player.pause_with_source(Source::Automatic)?;

                if let Some(volume) = config.player.volume {
                    player.volume_with_source(Source::Automatic, volume)?;
                }
            }
        }

        let futures = future::try_join_all(futures);
        future::try_join(future.run(settings), futures)
            .await
            .map(|_| ())
    };

    Ok((parent_player, future))
}

/// Events emitted by the player.
#[derive(Debug, Clone)]
pub enum Event {
    /// Player is empty.
    Empty,
    /// Player is playing the given song.
    Playing(bool, Arc<Item>),
    /// Player is pausing.
    Pausing,
    /// queue was modified in some way.
    Modified,
    /// player has not been configured.
    NotConfigured,
    /// Player is detached.
    Detached,
}

/// Information on current song.
#[derive(Debug, Clone)]
pub struct Song {
    pub item: Arc<Item>,
    /// Since the last time it was unpaused, what was the initial elapsed duration.
    elapsed: Duration,
    /// When the current song started playing.
    started_at: Option<Instant>,
}

impl Song {
    /// Create a new current song.
    pub fn new(item: Arc<Item>, elapsed: Duration) -> Self {
        Song {
            item,
            elapsed,
            started_at: None,
        }
    }

    /// Test if the two songs reference roughly the same song.
    pub fn is_same(&self, song: &Song) -> bool {
        if self.item.track_id != song.item.track_id {
            return false;
        }

        let a = self.elapsed();
        let b = song.elapsed();
        let diff = if a > b { a - b } else { b - a };

        if diff.as_secs() > 5 {
            return false;
        }

        true
    }

    /// Convert a playback information into a Song struct.
    pub fn from_playback(playback: &api::spotify::FullPlayingContext) -> Option<Self> {
        let progress_ms = playback.progress_ms.unwrap_or_default();

        let track = match playback.item.clone() {
            Some(track) => track,
            _ => return None,
        };

        let track_id: TrackId = match str::parse(&track.id) {
            Ok(track_id) => track_id,
            Err(_) => return None,
        };

        let elapsed = Duration::from_millis(progress_ms as u64);
        let duration = Duration::from_millis(track.duration_ms.into());

        let item = Arc::new(Item {
            track_id,
            track: Track::Spotify { track },
            user: None,
            duration,
        });

        let mut song = Song::new(item, elapsed);

        if playback.is_playing {
            song.play();
        } else {
            song.pause();
        }

        Some(song)
    }

    /// Get the deadline for when this song will end, assuming it is currently playing.
    pub fn deadline(&self) -> Instant {
        Instant::now() + self.remaining()
    }

    /// Duration of the current song.
    pub fn duration(&self) -> Duration {
        self.item.duration.clone()
    }

    /// Elapsed time on current song.
    ///
    /// Elapsed need to take started at into account.
    pub fn elapsed(&self) -> Duration {
        let when = self
            .started_at
            .as_ref()
            .and_then(|started_at| {
                let now = Instant::now();

                if now > *started_at {
                    Some(now - *started_at)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        when.checked_add(self.elapsed.clone()).unwrap_or_default()
    }

    /// Remaining time of the current song.
    pub fn remaining(&self) -> Duration {
        self.item
            .duration
            .checked_sub(self.elapsed())
            .unwrap_or_default()
    }

    /// Get serializable data for this item.
    pub fn data(&self, state: State) -> Result<CurrentData<'_>, Error> {
        let artists = self.item.track.artists();

        Ok(CurrentData {
            paused: state != State::Playing,
            track_id: &self.item.track_id,
            name: self.item.track.name(),
            artists,
            user: self.item.user.as_ref().map(|s| s.as_str()),
            duration: utils::digital_duration(&self.item.duration),
            elapsed: utils::digital_duration(&self.elapsed()),
        })
    }

    /// Check if the song is currently playing.
    pub fn state(&self) -> State {
        match self.started_at.is_some() {
            true => State::Playing,
            false => State::Paused,
        }
    }

    /// Get the player kind for the current song.
    pub fn player(&self) -> PlayerKind {
        match self.item.track_id {
            TrackId::Spotify(..) => PlayerKind::Spotify,
            TrackId::YouTube(..) => PlayerKind::YouTube,
        }
    }

    /// Set the started_at time to now.
    /// For safety, update the current `elapsed` time based on any prior `started_at`.
    pub fn play(&mut self) {
        let duration = self.take_started_at();
        self.elapsed += duration;
        self.started_at = Some(Instant::now());
    }

    /// Update the elapsed time based on when this song was started.
    pub fn pause(&mut self) {
        let duration = self.take_started_at();
        self.elapsed += duration;
    }

    /// Take the current started_at as a duration and leave it as None.
    fn take_started_at(&mut self) -> Duration {
        let started_at = match self.started_at.take() {
            Some(started_at) => started_at,
            None => return Default::default(),
        };

        let now = Instant::now();

        if now < started_at {
            return Default::default();
        }

        now - started_at
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrentData<'a> {
    paused: bool,
    track_id: &'a TrackId,
    name: String,
    artists: Option<String>,
    user: Option<&'a str>,
    duration: String,
    elapsed: String,
}

/// A handler for the player.
#[derive(Clone)]
pub struct Player {
    device: self::connect::ConnectDevice,
    queue: Queue,
    db: db::Database,
    max_queue_length: Arc<RwLock<u32>>,
    max_songs_per_user: Arc<RwLock<u32>>,
    duplicate_duration: Arc<RwLock<utils::Duration>>,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    commands_tx: mpsc::UnboundedSender<Command>,
    bus: Arc<RwLock<Bus<Event>>>,
    volume: Arc<RwLock<u32>>,
    /// The current song that is loaded.
    song: Arc<RwLock<Option<Song>>>,
    /// Theme songs.
    themes: db::Themes,
    /// Player is closed for more requests.
    closed: Arc<RwLock<Option<Option<Arc<String>>>>>,
}

impl Player {
    /// The client components of the player.
    pub fn client(&self) -> PlayerClient {
        PlayerClient {
            device: self.device.clone(),
            db: self.db.clone(),
            queue: self.queue.clone(),
            thread_pool: Arc::new(ThreadPool::new()),
            max_queue_length: self.max_queue_length.clone(),
            max_songs_per_user: self.max_songs_per_user.clone(),
            duplicate_duration: self.duplicate_duration.clone(),
            spotify: self.spotify.clone(),
            youtube: self.youtube.clone(),
            commands_tx: self.commands_tx.clone(),
            bus: self.bus.clone(),
            volume: Arc::clone(&self.volume),
            song: self.song.clone(),
            themes: self.themes.clone(),
            closed: self.closed.clone(),
        }
    }

    /// Get a receiver for player events.
    pub fn add_rx(&self) -> BusReader<Event> {
        self.bus.write().add_rx()
    }
}

/// All parts of a Player that can be shared between threads.
#[derive(Clone)]
pub struct PlayerClient {
    device: self::connect::ConnectDevice,
    db: db::Database,
    queue: Queue,
    thread_pool: Arc<ThreadPool>,
    max_queue_length: Arc<RwLock<u32>>,
    max_songs_per_user: Arc<RwLock<u32>>,
    duplicate_duration: Arc<RwLock<utils::Duration>>,
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    commands_tx: mpsc::UnboundedSender<Command>,
    bus: Arc<RwLock<Bus<Event>>>,
    /// Song volume.
    volume: Arc<RwLock<u32>>,
    /// Song song that is loaded.
    song: Arc<RwLock<Option<Song>>>,
    /// Theme songs.
    themes: db::Themes,
    /// Player is closed for more requests.
    closed: Arc<RwLock<Option<Option<Arc<String>>>>>,
}

impl PlayerClient {
    /// Get a receiver for player events.
    pub fn add_rx(&self) -> BusReader<Event> {
        self.bus.write().add_rx()
    }

    /// Synchronize playback with the given song.
    fn play_sync(&self, song: Option<Song>) -> Result<(), Error> {
        self.send(Command::Sync { song })
    }

    /// Get the current device.
    pub fn current_device(&self) -> Option<api::spotify::Device> {
        self.device.current_device()
    }

    /// List all available devices.
    pub async fn list_devices(&self) -> Result<Vec<api::spotify::Device>, Error> {
        self.device.list_devices().await
    }

    /// External call to set device.
    ///
    /// Should always notify the player to change.
    pub fn set_device(&self, device: api::spotify::Device) -> Option<api::spotify::Device> {
        self.device.set_device(Some(device))
    }

    /// Clear the current device.
    pub fn clear_device(&self) -> Option<api::spotify::Device> {
        self.device.set_device(None)
    }

    /// Send the given command.
    fn send(&self, command: Command) -> Result<(), Error> {
        self.commands_tx
            .unbounded_send(command)
            .map_err(|_| format_err!("failed to send command"))
    }

    /// Get the next N songs in queue.
    pub fn list(&self) -> Vec<Arc<Item>> {
        let song = self.song.read();
        let queue = self.queue.queue.read();

        song.as_ref()
            .map(|c| c.item.clone())
            .into_iter()
            .chain(queue.iter().cloned())
            .collect()
    }

    /// Promote the given song to the head of the queue.
    pub fn promote_song(&self, user: &str, n: usize) -> Option<Arc<Item>> {
        let promoted = self.queue.promote_song(user, n);

        if promoted.is_some() {
            self.modified();
        }

        promoted
    }

    /// Toggle playback.
    pub fn toggle(&self) -> Result<(), Error> {
        self.send(Command::Toggle(Source::Manual))
    }

    /// Start playback.
    pub fn play(&self) -> Result<(), Error> {
        self.send(Command::Play(Source::Manual))
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<(), Error> {
        self.pause_with_source(Source::Manual)
    }

    /// Pause playback.
    pub fn pause_with_source(&self, source: Source) -> Result<(), Error> {
        self.send(Command::Pause(source))
    }

    /// Skip the current song.
    pub fn skip(&self) -> Result<(), Error> {
        self.send(Command::Skip(Source::Manual))
    }

    /// Update volume of the player.
    pub fn volume(&self, volume: u32) -> Result<(), Error> {
        self.send(Command::Volume(Source::Manual, u32::min(100u32, volume)))
    }

    /// Update volume of the player with a custom source.
    pub fn volume_with_source(&self, source: Source, volume: u32) -> Result<(), Error> {
        self.send(Command::Volume(source, u32::min(100u32, volume)))
    }

    /// Get the current volume.
    pub fn current_volume(&self) -> u32 {
        *self.volume.read()
    }

    /// Close the player from more requests.
    pub fn close(&self, reason: Option<String>) {
        *self.closed.write() = Some(reason.map(Arc::new));
    }

    /// Open the player.
    pub fn open(&self) {
        *self.closed.write() = None;
    }

    /// Search for a track.
    pub async fn search_track(&self, q: String) -> Result<Option<TrackId>, Error> {
        if q.starts_with("youtube:") {
            let q = q.trim_start_matches("youtube:").to_string();
            let results = self.youtube.search(q).await?;

            let result = results.items.into_iter().filter(|r| match r.id.kind {
                api::youtube::Kind::Video => true,
                _ => false,
            });

            let mut result = result.flat_map(|r| r.id.video_id);
            return Ok(result.next().map(TrackId::YouTube));
        }

        let q = if q.starts_with("spotify:") {
            q.trim_start_matches("spotify:").to_string()
        } else {
            q
        };

        let page = self.spotify.search_track(q).await?;

        match page.items.into_iter().next() {
            Some(track) => match SpotifyId::from_base62(&track.id) {
                Ok(track_id) => Ok(Some(TrackId::Spotify(track_id))),
                Err(_) => bail!("search result returned malformed id"),
            },
            None => Ok(None),
        }
    }

    /// Play a theme track.
    pub async fn play_theme(&self, channel: String, name: String) -> Result<(), PlayThemeError> {
        let theme = match self.themes.get(&channel, &name) {
            Some(theme) => theme,
            None => return Err(PlayThemeError::NoSuchTheme),
        };

        let duration = theme.end.clone().map(|o| o.as_duration());

        let item = convert_item(
            self.spotify.clone(),
            self.youtube.clone(),
            None,
            theme.track_id.clone(),
            duration,
        )
        .await
        .map_err(|e| PlayThemeError::Error(e.into()))?;

        let item = Arc::new(item);
        let duration = theme.start.as_duration();

        self.commands_tx
            .unbounded_send(Command::Inject(Source::Manual, item, duration))
            .map_err(|e| PlayThemeError::Error(e.into()))?;

        Ok(())
    }

    /// Add the given track to the queue.
    ///
    /// Returns the item added.
    pub async fn add_track(
        &self,
        currency: Option<Currency>,
        channel: String,
        user: String,
        track_id: TrackId,
        is_moderator: bool,
        max_duration: Option<utils::Duration>,
        min_currency: Option<i64>,
    ) -> Result<(usize, Arc<Item>), AddTrackError> {
        let (user_count, len) = {
            let queue_inner = self.queue.queue.read();
            let len = queue_inner.len();

            if !is_moderator {
                if let Some(reason) = self.closed.read().as_ref() {
                    return Err(AddTrackError::PlayerClosed(reason.clone()));
                }

                let max_queue_length = *self.max_queue_length.read();

                // NB: moderator is allowed to violate max queue length.
                if len >= max_queue_length as usize {
                    return Err(AddTrackError::QueueFull);
                }

                let duplicate_duration = self.duplicate_duration.read().clone();

                if !duplicate_duration.is_empty() {
                    if let Some(last) = self
                        .queue
                        .last_song_within(&track_id, duplicate_duration.clone())
                        .map_err(AddTrackError::Error)?
                    {
                        let added_at = DateTime::from_utc(last.added_at, Utc);

                        return Err(AddTrackError::Duplicate(
                            added_at,
                            last.user,
                            duplicate_duration.as_std(),
                        ));
                    }
                }
            }

            let mut user_count = 0;

            for (index, i) in queue_inner.iter().enumerate() {
                if i.track_id == track_id {
                    return Err(AddTrackError::QueueContainsTrack(index));
                }

                if i.user.as_ref().map(|u| *u == user).unwrap_or_default() {
                    user_count += 1;
                }
            }

            (user_count, len)
        };

        if !is_moderator {
            if let Some(min_currency) = min_currency {
                let currency = match currency.as_ref() {
                    Some(currency) => currency,
                    None => return Err(AddTrackError::NoCurrency),
                };

                let balance = currency
                    .balance_of(channel, user.clone())
                    .await
                    .map_err(AddTrackError::Error)?
                    .unwrap_or_default();

                if balance < min_currency {
                    return Err(AddTrackError::NotEnoughCurrency {
                        balance,
                        required: min_currency,
                    });
                }
            }
        }

        let max_songs_per_user = *self.max_songs_per_user.read();

        // NB: moderator is allowed to add more songs.
        if !is_moderator && user_count >= max_songs_per_user {
            return Err(AddTrackError::TooManyUserTracks(max_songs_per_user));
        }

        let mut item = convert_item(
            self.spotify.clone(),
            self.youtube.clone(),
            Some(user),
            track_id,
            None,
        )
        .await
        .map_err(|e| AddTrackError::Error(e.into()))?;

        if let Some(max_duration) = max_duration {
            let max_duration = max_duration.as_std();

            if item.duration > max_duration {
                item.duration = max_duration;
            }
        }

        let item = Arc::new(item);

        self.queue
            .push_back(item.clone())
            .await
            .map_err(|e| AddTrackError::Error(e.into()))?;

        self.commands_tx
            .unbounded_send(Command::Modified(Source::Manual))
            .map_err(|e| AddTrackError::Error(e.into()))?;

        Ok((len, item))
    }

    /// Remove the first track in the queue.
    pub fn remove_first(&self) -> Result<Option<Arc<Item>>, Error> {
        Ok(None)
    }

    pub fn purge(&self) -> Result<Vec<Arc<Item>>, Error> {
        let purged = self.queue.purge()?;

        if !purged.is_empty() {
            self.modified();
        }

        Ok(purged)
    }

    /// Remove the item at the given position.
    pub fn remove_at(&self, n: usize) -> Result<Option<Arc<Item>>, Error> {
        let removed = self.queue.remove_at(n)?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Remove the first track in the queue.
    pub fn remove_last(&self) -> Result<Option<Arc<Item>>, Error> {
        let removed = self.queue.remove_last()?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Remove the last track by the given user.
    pub fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>, Error> {
        let removed = self.queue.remove_last_by_user(user)?;

        if removed.is_some() {
            self.modified();
        }

        Ok(removed)
    }

    /// Find the next item that matches the given predicate and how long until it plays.
    pub fn find(&self, mut predicate: impl FnMut(&Item) -> bool) -> Option<(Duration, Arc<Item>)> {
        let mut duration = Duration::default();

        if let Some(c) = self.song.read().as_ref() {
            if predicate(&c.item) {
                return Some((Default::default(), c.item.clone()));
            }

            duration += c.remaining();
        }

        let queue = self.queue.queue.read();

        for item in &*queue {
            if predicate(item) {
                return Some((duration, item.clone()));
            }

            duration += item.duration;
        }

        None
    }

    /// Get the length in number of items and total number of seconds in queue.
    pub fn length(&self) -> (usize, Duration) {
        let mut count = 0;
        let mut duration = Duration::default();

        if let Some(item) = self.song.read().as_ref() {
            duration += item.remaining();
            count += 1;
        }

        let queue = self.queue.queue.read();

        for item in &*queue {
            duration += item.duration;
        }

        count += queue.len();
        (count, duration)
    }

    /// Get the current song, if it is set.
    pub fn current(&self) -> Option<Song> {
        self.song.read().clone()
    }

    /// Indicate that the queue has been modified.
    fn modified(&self) {
        if let Err(e) = self
            .commands_tx
            .unbounded_send(Command::Modified(Source::Manual))
        {
            log::error!("failed to send queue modified notification: {}", e);
        }
    }
}

/// Error raised when failing to play a theme song.
pub enum PlayThemeError {
    /// No such theme song.
    NoSuchTheme,
    /// Other generic error happened.
    Error(Error),
}

/// Error raised when trying to add track.
pub enum AddTrackError {
    /// Queue is full.
    QueueFull,
    /// Queue already contains track.
    QueueContainsTrack(usize),
    /// Too many user tracks.
    TooManyUserTracks(u32),
    /// Player has been closed from adding more tracks to the queue with an optional reason.
    PlayerClosed(Option<Arc<String>>),
    /// Duplicate song that was added at the specified time by the specified user.
    Duplicate(DateTime<Utc>, Option<String>, Duration),
    /// No currency configured.
    NoCurrency,
    /// Not enough currency to request songs.
    NotEnoughCurrency { required: i64, balance: i64 },
    /// Other generic error happened.
    Error(Error),
}

/// The backend of a words store.
pub trait Backend: Clone + Send + Sync {
    /// List all counters in backend.
    fn list(&self) -> Result<Vec<db::models::Song>, Error>;

    /// Insert the given song into the backend.
    fn push_back(&self, song: &db::models::AddSong) -> Result<(), Error>;

    /// Remove the song, but only log on issues.
    fn remove_song_log(&self, track_id: &TrackId) {
        match self.remove_song(track_id) {
            Err(e) => log::warn!("{}: failed to remove song from database: {}", track_id, e),
            Ok(false) => log::warn!("{}: no songs removed from database", track_id),
            Ok(true) => {}
        }
    }

    /// Remove the song with the given ID.
    fn remove_song(&self, track_id: &TrackId) -> Result<bool, Error>;

    /// Purge the songs database, but only log on issues.
    fn song_purge_log(&self) -> Option<usize> {
        match self.song_purge() {
            Err(e) => {
                log::warn!("failed to purge songs from database: {}", e);
                None
            }
            Ok(n) => Some(n),
        }
    }

    /// Purge the songs database and return the number of items removed.
    fn song_purge(&self) -> Result<usize, Error>;

    /// Purge the songs database, but only log on issues.
    fn promote_song_log(&self, user: &str, track_id: &TrackId) -> Option<bool> {
        match self.promote_song(user, track_id) {
            Err(e) => {
                log::warn!("failed to promote song `{}` in database: {}", track_id, e);
                None
            }
            Ok(n) => Some(n),
        }
    }

    /// Promote the track with the given ID.
    fn promote_song(&self, user: &str, track_id: &TrackId) -> Result<bool, Error>;

    /// Test if the song has been played within a given duration.
    fn last_song_within(
        &self,
        track_id: &TrackId,
        duration: utils::Duration,
    ) -> Result<Option<db::models::Song>, Error>;
}

/// The playback queue.
#[derive(Clone)]
struct Queue {
    db: db::Database,
    queue: Arc<RwLock<VecDeque<Arc<Item>>>>,
    thread_pool: Arc<ThreadPool>,
}

impl Queue {
    /// Construct a new queue.
    pub fn new(db: db::Database) -> Self {
        Self {
            db,
            queue: Arc::new(RwLock::new(Default::default())),
            thread_pool: Arc::new(ThreadPool::new()),
        }
    }

    /// Check ifa song has been queued within the specified period of time.
    pub fn last_song_within(
        &self,
        track_id: &TrackId,
        duration: utils::Duration,
    ) -> Result<Option<db::models::Song>, Error> {
        self.db.last_song_within(track_id, duration)
    }

    /// Get the front of the queue.
    pub fn front(&self) -> Option<Arc<Item>> {
        self.queue.read().front().cloned()
    }

    /// Pop the front of the queue.
    pub fn pop_front(
        &self,
    ) -> impl Future<Output = Result<Option<Arc<Item>>, Error>> + Send + 'static {
        let db = self.db.clone();
        let queue = self.queue.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            if let Some(item) = queue.write().pop_front() {
                db.remove_song_log(&item.track_id);
            }

            Ok(None)
        }));

        future.compat()
    }

    /// Push item to back of queue.
    pub async fn push_back(&self, item: Arc<Item>) -> Result<(), Error> {
        let db = self.db.clone();
        let queue = self.queue.clone();

        let future = self.thread_pool.spawn_handle(future01::lazy(move || {
            db.push_back(&db::models::AddSong {
                track_id: item.track_id.clone(),
                added_at: Utc::now().naive_utc(),
                user: item.user.clone(),
            })?;

            queue.write().push_back(item);
            Ok(())
        }));

        future.compat().await
    }

    /// Purge the song queue.
    pub fn purge(&self) -> Result<Vec<Arc<Item>>, Error> {
        let mut q = self.queue.write();

        if q.is_empty() {
            return Ok(vec![]);
        }

        let purged = std::mem::replace(&mut *q, VecDeque::new())
            .into_iter()
            .collect();
        self.db.song_purge_log();
        Ok(purged)
    }

    /// Remove the item at the given position.
    pub fn remove_at(&self, n: usize) -> Result<Option<Arc<Item>>, Error> {
        let mut q = self.queue.write();

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(item) = q.remove(n) {
            self.db.remove_song_log(&item.track_id);
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element.
    pub fn remove_last(&self) -> Result<Option<Arc<Item>>, Error> {
        let mut q = self.queue.write();

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(item) = q.pop_back() {
            self.db.remove_song_log(&item.track_id);
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element by user.
    pub fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>, Error> {
        let mut q = self.queue.write();

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(position) = q
            .iter()
            .rposition(|i| i.user.as_ref().map(|u| u == user).unwrap_or_default())
        {
            if let Some(item) = q.remove(position) {
                self.db.remove_song_log(&item.track_id);
                return Ok(Some(item));
            }
        }

        Ok(None)
    }

    /// Promote the given song.
    pub fn promote_song(&self, user: &str, n: usize) -> Option<Arc<Item>> {
        let mut q = self.queue.write();

        // OK, but song doesn't exist or index is out of bound.
        if q.is_empty() || n >= q.len() {
            return None;
        }

        if let Some(removed) = q.remove(n) {
            q.push_front(removed);
        }

        if let Some(item) = q.get(0).cloned() {
            self.db.promote_song_log(user, &item.track_id);
            return Some(item);
        }

        None
    }

    /// Push item to back of queue without going through the database.
    fn push_back_queue(&self, item: Arc<Item>) {
        self.queue.write().push_back(item);
    }
}

/// Mixer decides what song to play next.
pub struct Mixer {
    /// Persistent queue to take songs from.
    queue: Queue,
    /// A song that has been sidelined by another song.
    sidelined: VecDeque<Song>,
    /// Currently loaded fallback items.
    fallback_items: Vec<Arc<Item>>,
    /// Items ordered in the reverse way they are meant to be played.
    fallback_queue: VecDeque<Arc<Item>>,
}

impl Mixer {
    /// The minimum size of the fallback queue.
    const FALLBACK_QUEUE_SIZE: usize = 10;
    /// Get next song to play.
    ///
    /// Will shuffle all fallback items and add them to a queue to avoid playing the same song twice.
    fn next_fallback_item(&mut self) -> Option<Song> {
        use rand::seq::SliceRandom;

        if self.fallback_items.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();

        while self.fallback_queue.len() < Self::FALLBACK_QUEUE_SIZE {
            let mut extension = self.fallback_items.clone();
            extension.shuffle(&mut rng);
            self.fallback_queue.extend(extension);
        }

        let item = self.fallback_queue.pop_front()?;
        Some(Song::new(item, Default::default()))
    }

    /// Get the next song that should be played.
    ///
    /// This takes into account:
    /// If there are any songs to be injected (e.g. theme songs).
    /// If there are any songs that have been sidelines by injected songs.
    /// If there are any songs in the queue.
    ///
    /// Finally, if there are any songs to fall back to.
    async fn next_song(&mut self) -> Result<Option<Song>, Error> {
        if let Some(song) = self.sidelined.pop_front() {
            return Ok(Some(song));
        }

        // Take next from queue.
        if let Some(item) = self.queue.front() {
            let _ = self.queue.pop_front().await?;
            return Ok(Some(Song::new(item, Default::default())));
        }

        if self.fallback_items.is_empty() {
            log::warn!("there are no fallback songs available");
            return Ok(None);
        }

        Ok(self.next_fallback_item())
    }
}

pub struct EventBus {
    bus: Arc<RwLock<Bus<Event>>>,
}

impl EventBus {
    /// Broadcast an event from the player.
    fn broadcast(&self, event: Event) {
        let mut b = self.bus.write();

        if let Err(e) = b.try_broadcast(event) {
            log::error!("failed to broadcast player event: {:?}", e);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Spotify,
    YouTube,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Playing,
    Paused,
    // initial undefined state.
    None,
}

/// Future associated with driving audio playback.
pub struct PlaybackFuture {
    config: Arc<config::Config>,
    spotify: Arc<api::Spotify>,
    connect_player: self::connect::ConnectPlayer,
    youtube_player: self::youtube::YouTubePlayer,
    commands: mpsc::UnboundedReceiver<Command>,
    bus: EventBus,
    mixer: Mixer,
    /// We are currently playing.
    state: State,
    /// Current player kind.
    player: PlayerKind,
    /// Player is detached.
    detached: bool,
    /// Stream of settings if the player is detached.
    detached_stream: settings::Stream<bool>,
    /// Song volume.
    volume: Arc<RwLock<u32>>,
    /// Song that is currently loaded.
    song: Arc<RwLock<Option<Song>>>,
    /// Path to write current song.
    song_file: Option<SongFile>,
    /// Song config.
    song_switch_feedback: Arc<RwLock<bool>>,
    /// Optional stream indicating that we want to send a song update on the global bus.
    song_update_interval: Option<timer::Interval>,
    /// Stream for when song update interval is updated.
    song_update_interval_stream: settings::Stream<utils::Duration>,
    /// Notifier to use when sending song updates.
    global_bus: Arc<bus::Bus<bus::Global>>,
    /// Timeout for end of song.
    timeout: Option<timer::Delay>,
}

impl PlaybackFuture {
    /// Run the playback future.
    pub async fn run(mut self, settings: settings::Settings) -> Result<(), Error> {
        let song_file = settings.scoped("song-file");

        let (mut path_stream, path) = song_file
            .stream("path")
            .or_else(|| self.config.current_song.path.clone())
            .optional()?;

        let default_template = Some(
            self.config
                .current_song
                .template
                .clone()
                .map(Ok)
                .unwrap_or_else(|| Template::compile(DEFAULT_CURRENT_SONG_TEMPLATE))?,
        );

        let default_stopped_template = Some(
            self.config
                .current_song
                .not_playing
                .clone()
                .map(Ok)
                .unwrap_or_else(|| Template::compile(DEFAULT_CURRENT_SONG_STOPPED_TEMPLATE))?,
        );

        let (mut template_stream, template) = song_file
            .stream("template")
            .or(default_template)
            .optional()?;

        let (mut stopped_template_stream, stopped_template) = song_file
            .stream("stopped-template")
            .or(default_stopped_template)
            .optional()?;

        let (mut update_interval_stream, update_interval) = song_file
            .stream("update-interval")
            .or_with(self.config.current_song.update_interval.clone())?;

        let (mut enabled_stream, enabled) = song_file.stream("enabled").or_default()?;

        let (mut fallback_stream, fallback) = settings.stream("fallback-uri").optional()?;
        self.update_fallback_items(fallback.clone()).await;

        let mut song_file = SongFileBuilder::default();
        song_file.enabled = enabled;
        song_file.path = path;
        song_file.template = template;
        song_file.stopped_template = stopped_template;
        song_file.update_interval = update_interval;
        song_file.init(&mut self.song_file);

        loop {
            let mut song_file_update = self.song_file.as_mut().map(|u| &mut u.update_interval);

            futures::select! {
                fallback = fallback_stream.select_next_some() => {
                    self.update_fallback_items(fallback).await;
                }
                /* current song */
                update = enabled_stream.select_next_some() => {
                    song_file.enabled = update;
                    song_file.init(&mut self.song_file);
                }
                update = path_stream.select_next_some() => {
                    song_file.path = update;
                    song_file.init(&mut self.song_file);
                }
                update = template_stream.select_next_some() => {
                    song_file.template = update;
                    song_file.init(&mut self.song_file);
                }
                update = stopped_template_stream.select_next_some() => {
                    song_file.stopped_template = update;
                    song_file.init(&mut self.song_file);
                }
                update = update_interval_stream.select_next_some() => {
                    song_file.update_interval = update;
                    song_file.init(&mut self.song_file);
                }
                update = song_file_update.select_next_some() => {
                    let _ =  update?;
                    let song = self.song.read();
                    self.update_song_file(song.as_ref());
                }
                /* player */
                timeout = self.timeout.current() => {
                    timeout?;
                    self.end_of_track().await?;
                }
                update = self.detached_stream.select_next_some() => {
                    if update {
                        self.detach()?;
                    }

                    self.detached = update;
                }
                value = self.song_update_interval_stream.select_next_some() => {
                    self.song_update_interval = match value.is_empty() {
                        true => None,
                        false => Some(timer::Interval::new_interval(value.as_std())),
                    };
                }
                update = self.song_update_interval.select_next_some() => {
                    let _ = update?;
                    let song = self.song.read();

                    if let State::Playing = self.state {
                        self.global_bus
                            .send(bus::Global::song_progress(song.as_ref()));

                        if let Some(song) = song.as_ref() {
                            if let TrackId::YouTube(ref id) = song.item.track_id {
                                self.youtube_player.tick(song.elapsed(), song.duration(), id.to_string());
                            }
                        }
                    }
                }
                event = self.connect_player.select_next_some() => {
                    self.handle_player_event(event?).await?;
                }
                command = self.commands.select_next_some() => {
                    self.command(command).await?;
                }
            }
        }
    }

    /// Update fallback items based on an URI.
    async fn update_fallback_items(&mut self, uri: Option<String>) {
        let result = match uri.as_ref() {
            Some(uri) => {
                let result = match parse_playlist_id(uri) {
                    Some(id) => Self::playlist_to_items(&self.spotify, id.to_string()).await,
                    None => Self::playlist_to_items(&self.spotify, uri.clone()).await,
                };

                match result {
                    Ok((name, items)) => Ok((Some(name), items)),
                    Err(e) => {
                        log::warn!(
                            "Failed to load playlist `{}`, \
                             falling back to library: {}",
                            uri,
                            e
                        );
                        Self::songs_to_items(&self.spotify)
                            .await
                            .map(|items| (None, items))
                    }
                }
            }
            None => Self::songs_to_items(&self.spotify)
                .await
                .map(|items| (None, items)),
        };

        let (what, items) = match result {
            Ok(result) => result,
            Err(e) => {
                log_err!(e, "Failed to configure fallback items");
                return;
            }
        };

        let what = what
            .as_ref()
            .map(|u| format!("\"{}\" playlist", u))
            .unwrap_or_else(|| String::from("your library"));

        log::info!(
            "Updated fallback queue with {} items from {}.",
            items.len(),
            what
        );

        self.mixer.fallback_items = items;
        self.mixer.fallback_queue.clear();

        fn parse_playlist_id<'a>(s: &'a str) -> Option<&'a str> {
            let mut p = s.split(":");

            match (p.next(), p.next(), p.next(), p.next(), p.next()) {
                (Some("spotify"), Some("user"), Some(_user), Some("playlist"), Some(id)) => {
                    Some(id)
                }
                _ => None,
            }
        }
    }

    /// Convert a playlist into items.
    async fn playlist_to_items(
        spotify: &Arc<api::Spotify>,
        playlist: String,
    ) -> Result<(String, Vec<Arc<Item>>), Error> {
        let mut items = Vec::new();

        let playlist = spotify.playlist(playlist).await?;
        let name = playlist.name.to_string();

        for playlist_track in spotify.page_as_stream(playlist.tracks).try_concat().await? {
            let track = playlist_track.track;

            let track_id = TrackId::Spotify(
                SpotifyId::from_base62(&track.id)
                    .map_err(|_| format_err!("bad spotify id: {}", track.id))?,
            );

            let duration = Duration::from_millis(track.duration_ms.into());

            items.push(Arc::new(Item {
                track_id,
                track: Track::Spotify { track },
                user: None,
                duration,
            }));
        }

        Ok((name, items))
    }

    /// Convert all songs of a user into items.
    async fn songs_to_items(spotify: &Arc<api::Spotify>) -> Result<Vec<Arc<Item>>, Error> {
        let mut items = Vec::new();

        for added_song in spotify.my_tracks_stream().try_concat().await? {
            let track = added_song.track;

            let track_id = TrackId::Spotify(
                SpotifyId::from_base62(&track.id)
                    .map_err(|_| format_err!("bad spotify id: {}", track.id))?,
            );

            let duration = Duration::from_millis(track.duration_ms.into());

            items.push(Arc::new(Item {
                track_id,
                track: Track::Spotify { track },
                user: None,
                duration,
            }));
        }

        Ok(items)
    }

    /// Notify a change in the current song.
    fn notify_song_change(&self, song: Option<&Song>) -> Result<(), Error> {
        self.global_bus.send(bus::Global::song(song)?);
        self.update_song_file(song);
        Ok(())
    }

    /// Write the current song.
    fn write_song(&self, song: Option<Song>) -> Result<(), Error> {
        *self.song.write() = song;
        Ok(())
    }

    /// Write current song. Log any errors.
    ///
    /// MUST NOT be called when self.song is locked.
    fn update_song_file(&self, song: Option<&Song>) {
        let current_song = match self.song_file.as_ref() {
            Some(current_song) => current_song,
            None => return,
        };

        let result = match song {
            Some(song) => current_song.write(song, self.state),
            None => current_song.blank(),
        };

        if let Err(e) = result {
            log::warn!(
                "failed to write current song: {}: {}",
                current_song.path.display(),
                e
            );
        }
    }

    /// Switch the current player and send the appropriate play commands.
    async fn switch_current_player(&mut self, player: PlayerKind) {
        use self::PlayerKind::*;

        match (self.player, player) {
            (Spotify, Spotify) => (),
            (YouTube, YouTube) => (),
            (Spotify, _) | (None, YouTube) => {
                let result = self.connect_player.stop().await;

                if let Err(self::connect::CommandError::NoDevice) = result {
                    self.bus.broadcast(Event::NotConfigured);
                }
            }
            (YouTube, _) | (None, Spotify) => self.youtube_player.stop(),
            (None, None) => (),
        }

        self.player = player;
    }

    /// Send a pause command to the appropriate player.
    async fn send_pause_command(&mut self) {
        match self.player {
            PlayerKind::Spotify => {
                log::trace!("pausing spotify player");

                let result = self.connect_player.pause().await;

                if let Err(self::connect::CommandError::NoDevice) = result {
                    self.bus.broadcast(Event::NotConfigured);
                }
            }
            PlayerKind::YouTube => {
                log::trace!("pausing youtube player");
                self.youtube_player.pause();
            }
            _ => (),
        }
    }

    /// Play the given song.
    async fn send_play_command(&mut self, song: Song) {
        match song.item.track_id.clone() {
            TrackId::Spotify(id) => {
                let result = self.connect_player.play(song.elapsed(), id).await;

                if let Err(self::connect::CommandError::NoDevice) = result {
                    self.bus.broadcast(Event::NotConfigured);
                }
            }
            TrackId::YouTube(id) => self
                .youtube_player
                .play(song.elapsed(), song.duration(), id),
        }
    }

    /// Switch the player to the specified song without changing its state.
    async fn switch_to_song(&mut self, mut song: Option<Song>) -> Result<(), Error> {
        if let Some(song) = song.as_mut() {
            song.pause();
            self.switch_current_player(song.player()).await;
        } else {
            self.switch_current_player(PlayerKind::None).await;
        }

        self.write_song(song)?;
        Ok(())
    }

    /// Switch current song to the specified song.
    async fn play_song(&mut self, source: Source, mut song: Song) -> Result<(), Error> {
        song.play();

        if let Source::Manual = source {
            self.bus.broadcast(Event::Playing(
                *self.song_switch_feedback.read(),
                song.item.clone(),
            ));
        }

        self.timeout = Some(timer::Delay::new(song.deadline()));

        self.send_play_command(song.clone()).await;
        self.switch_current_player(song.player()).await;
        self.notify_song_change(Some(&song))?;
        self.write_song(Some(song))?;

        self.state = State::Playing;
        Ok(())
    }

    /// Resume playing a specific song.
    async fn resume_song(&mut self, source: Source, song: Song) -> Result<(), Error> {
        if let Source::Manual = source {
            self.bus.broadcast(Event::Playing(
                *self.song_switch_feedback.read(),
                song.item.clone(),
            ));
        }

        self.timeout = Some(timer::Delay::new(song.deadline()));

        self.send_play_command(song.clone()).await;
        self.switch_current_player(song.player()).await;
        self.notify_song_change(Some(&song))?;

        self.state = State::Playing;
        Ok(())
    }

    /// Detach the player.
    fn detach(&mut self) -> Result<(), Error> {
        // store the currently playing song in the sidelined slot.
        if let Some(mut song) = self.song.write().take() {
            song.pause();
            self.mixer.sidelined.push_back(song);
        }

        self.write_song(None)?;
        self.player = PlayerKind::None;
        self.state = State::None;
        self.timeout = None;
        Ok(())
    }

    /// Handle incoming command.
    async fn command(&mut self, command: Command) -> Result<(), Error> {
        use self::Command::*;

        if self.detached {
            log::trace!(
                "Ignoring: Command = {:?}, State = {:?}, Player = {:?}",
                command,
                self.state,
                self.player,
            );

            if let Source::Manual = command.source() {
                self.bus.broadcast(Event::Detached);
            }

            return Ok(());
        }

        log::trace!(
            "Processing: Command = {:?}, State = {:?}, Player = {:?}",
            command,
            self.state,
            self.player,
        );

        let command = match (command, self.state) {
            (Toggle(source), State::Paused) | (Toggle(source), State::None) => Play(source),
            (Toggle(source), State::Playing) => Pause(source),
            (command, _) => command,
        };

        match (command, self.state) {
            (Skip(source), _) => {
                log::trace!("skipping song");

                let song = self.mixer.next_song().await?;

                match (song, self.state) {
                    (Some(song), State::Playing) => self.play_song(source, song).await?,
                    (Some(song), _) => self.switch_to_song(Some(song)).await?,
                    (None, _) => {
                        if let Source::Manual = source {
                            self.bus.broadcast(Event::Empty);
                        }

                        self.switch_to_song(None).await?;
                        self.state = State::Paused;
                    }
                }
            }
            // initial pause
            (Pause(source), State::Playing) => {
                log::trace!("pausing player");

                self.send_pause_command().await;
                self.timeout = None;
                self.state = State::Paused;

                let mut song = self.song.write();

                if let Some(song) = song.as_mut() {
                    song.pause();
                }

                if let Source::Manual = source {
                    self.bus.broadcast(Event::Pausing);
                }

                self.notify_song_change(song.as_ref())?;
            }
            (Play(source), State::Paused) | (Play(source), State::None) => {
                log::trace!("starting player");

                let song = {
                    match self.song.write().as_mut() {
                        Some(song) => {
                            song.play();
                            Some(song.clone())
                        }
                        None => None,
                    }
                };

                // resume an existing song
                if let Some(song) = song {
                    self.resume_song(source, song.clone()).await?;
                    return Ok(());
                }

                // play the next song in queue.
                if let Some(song) = self.mixer.next_song().await? {
                    self.play_song(source, song).await?;
                } else {
                    if let Source::Manual = source {
                        self.bus.broadcast(Event::Empty);
                    }

                    self.write_song(None)?;
                    self.state = State::Paused;
                }
            }
            (Sync { song }, _) => {
                log::trace!("synchronize the state of the player with the current song");

                if let Some(s) = song.as_ref() {
                    if let State::Playing = s.state() {
                        self.timeout = Some(timer::Delay::new(s.deadline()));
                    }

                    self.switch_current_player(s.player()).await;
                    self.state = State::Playing;
                } else {
                    self.timeout = None;
                    self.switch_current_player(PlayerKind::None).await;
                    self.state = State::Paused;
                }

                self.write_song(song)?;
            }
            // queue was modified in some way
            (Modified(source), State::Playing) => {
                if self.song.read().is_none() {
                    if let Some(song) = self.mixer.next_song().await? {
                        self.play_song(source, song).await?;
                    }
                }

                self.bus.broadcast(Event::Modified);
            }
            (Volume(_, volume), _) => {
                let result = self.connect_player.volume(volume).await;

                if let Err(self::connect::CommandError::NoDevice) = result {
                    self.bus.broadcast(Event::NotConfigured);
                }

                self.youtube_player.volume(volume);
                *self.volume.write() = volume;
            }
            (Inject(source, item, offset), State::Playing) => {
                {
                    // store the currently playing song in the sidelined slot.
                    if let Some(mut song) = self.song.write().take() {
                        song.pause();
                        self.mixer.sidelined.push_back(song);
                    }
                }

                self.play_song(source, Song::new(item, offset)).await?;
            }
            _ => (),
        }

        Ok(())
    }

    /// We've reached the end of a track.
    async fn end_of_track(&mut self) -> Result<(), Error> {
        if self.detached {
            log::warn!("End of track called even though we are detached");
            return Ok(());
        }

        log::trace!("Song ended, loading next song...");

        if let Some(song) = self.mixer.next_song().await? {
            self.play_song(Source::Manual, song).await?;
        } else {
            self.bus.broadcast(Event::Empty);
            self.write_song(None)?;
        }

        Ok(())
    }

    /// Handle an event from the connect integration.
    async fn handle_player_event(&mut self, e: IntegrationEvent) -> Result<(), Error> {
        use self::IntegrationEvent::*;

        if self.detached {
            log::trace!(
                "Ignoring: IntegrationEvent = {:?}, State = {:?}, Player = {:?}",
                e,
                self.state,
                self.player,
            );

            return Ok(());
        }

        log::trace!(
            "Processing: IntegrationEvent = {:?}, State = {:?}, Player = {:?}",
            e,
            self.state,
            self.player,
        );

        match e {
            DeviceChanged => {
                if self.state != State::Playing {
                    return Ok(());
                }

                let volume = *self.volume.read();

                let (elapsed, duration, track_id) = {
                    let mut song = self.song.write();

                    let song = match song.as_mut() {
                        Some(song) => song,
                        None => return Ok(()),
                    };

                    // pause so that it can get unpaused later.
                    song.pause();
                    (song.elapsed(), song.duration(), song.item.track_id.clone())
                };

                match track_id {
                    TrackId::Spotify(id) => {
                        let c1 = self.connect_player.play(elapsed, id);
                        let c2 = self.connect_player.volume(volume);
                        let result = future::try_join(c1, c2).await;

                        if let Err(self::connect::CommandError::NoDevice) = result {
                            self.bus.broadcast(Event::NotConfigured);
                        }

                        self.switch_current_player(PlayerKind::Spotify).await;
                        self.state = State::Playing;
                    }
                    TrackId::YouTube(id) => {
                        self.youtube_player.play(elapsed, duration, id);
                        self.youtube_player.volume(volume);
                        self.switch_current_player(PlayerKind::YouTube).await;
                        self.state = State::Playing;
                    }
                }
            }
        }

        Ok(())
    }
}

/// Converts a track into an Item.
async fn convert_item(
    spotify: Arc<api::Spotify>,
    youtube: Arc<api::YouTube>,
    user: Option<String>,
    track_id: TrackId,
    duration_override: Option<Duration>,
) -> Result<Item, Error> {
    let (track, duration) = match track_id {
        TrackId::Spotify(ref id) => {
            let track_id_string = id.to_base62();
            let track = spotify.track(track_id_string).await?;
            let duration = Duration::from_millis(track.duration_ms.into());

            (Track::Spotify { track }, duration)
        }
        TrackId::YouTube(ref id) => {
            let id = id.to_string();
            let video_info = youtube.get_video_info(id.clone());
            let video = youtube
                .clone()
                .videos_by_id(id.clone(), String::from("contentDetails,snippet"));
            let (video_info, video) = future::try_join(video_info, video).await?;

            log::trace!("info = {:?}", video_info);

            let video = match video {
                Some(video) => video,
                None => bail!("no video found for id `{}`", id),
            };

            let content_details = video
                .content_details
                .as_ref()
                .ok_or_else(|| failure::format_err!("video does not have content details"))?;

            let duration = parse_youtube_duration(&content_details.duration)?;
            (Track::YouTube { video }, duration)
        }
    };

    let duration = match duration_override {
        Some(duration) => duration,
        None => duration,
    };

    return Ok(Item {
        track_id,
        track,
        user,
        duration,
    });

    fn parse_youtube_duration(duration: &str) -> Result<Duration, Error> {
        let duration = duration.trim_start_matches("PT");

        let (duration, hours) = match duration.find('H') {
            Some(index) => {
                let hours = str::parse::<u64>(&duration[..index])?;
                (&duration[(index + 1)..], hours)
            }
            None => (duration, 0u64),
        };

        let (duration, minutes) = match duration.find('M') {
            Some(index) => {
                let minutes = str::parse::<u64>(&duration[..index])?;
                (&duration[(index + 1)..], minutes)
            }
            None => (duration, 0u64),
        };

        let (_, mut seconds) = match duration.find('S') {
            Some(index) => {
                let seconds = str::parse::<u64>(&duration[..index])?;
                (&duration[(index + 1)..], seconds)
            }
            None => (duration, 0u64),
        };

        seconds += minutes * 60;
        seconds += hours * 3600;

        Ok(Duration::from_secs(seconds))
    }
}
