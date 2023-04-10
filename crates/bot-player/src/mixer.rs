use std::collections::VecDeque;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use common::models::{Item, Song, TrackId};
use common::Duration;
use tokio::sync::Mutex;
use tokio::sync::MutexGuard;

#[derive(Default)]
struct Fallback {
    /// Currently loaded fallback items.
    items: Vec<Arc<Item>>,
    /// Items ordered in the reverse way they are meant to be played.
    queue: VecDeque<Arc<Item>>,
}

/// Mixer decides what song to play next.
pub(super) struct Mixer {
    /// Database access.
    db: db::Database,
    /// In-memory queue.
    queue: Mutex<VecDeque<Arc<Item>>>,
    /// A song that has been sidelined by another song.
    sidelined: parking_lot::Mutex<VecDeque<Song>>,
    /// Fallback queue.
    fallback: Mutex<Fallback>,
    /// Keeping track of queue length.
    len: AtomicUsize,
}

impl Mixer {
    /// The minimum size of the fallback queue.
    const FALLBACK_QUEUE_SIZE: usize = 10;

    /// Construct a new mixer around the given queue.
    pub(super) fn new(db: db::Database) -> Self {
        Self {
            db,
            queue: Mutex::default(),
            sidelined: parking_lot::Mutex::default(),
            fallback: Mutex::default(),
            len: AtomicUsize::new(0),
        }
    }

    /// Initialize the queue from the database.
    #[tracing::instrument(skip_all)]
    pub(super) async fn initialize_queue(
        &self,
        spotify: &api::Spotify,
        youtube: &api::YouTube,
    ) -> Result<()> {
        // TODO: cache this value
        let streamer = spotify.me().await?;
        let market = streamer.country.as_deref();
        let mut queue = self.queue.lock().await;

        // Add tracks from database.
        for song in self.db.player_list().await? {
            let item = crate::convert_item(
                spotify,
                youtube,
                song.user.as_deref(),
                &song.track_id,
                None,
                market,
            )
            .await;

            match item {
                Ok(item) => {
                    queue.extend(item.map(Arc::new));
                }
                Err(error) => {
                    common::log_warn!(error, "Failed to convert database item");
                }
            }
        }

        self.len.store(queue.len(), Ordering::SeqCst);
        Ok(())
    }

    pub(super) fn len(&self) -> usize {
        self.len.load(Ordering::SeqCst)
    }

    /// List items in the queue.
    pub(super) async fn queue(&self) -> MutexGuard<'_, VecDeque<Arc<Item>>> {
        self.queue.lock().await
    }

    /// Push item to back of queue.
    pub(super) async fn push_back(&self, item: Arc<Item>) -> Result<()> {
        self.db
            .player_push_back(&db::models::AddSong {
                track_id: item.track_id().clone(),
                added_at: Utc::now().naive_utc(),
                user: item.user().cloned(),
            })
            .await?;

        self.queue.lock().await.push_back(item);
        self.len.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Purge the song queue.
    pub(super) async fn purge(&self) -> Result<Vec<Arc<Item>>> {
        let purged = self.queue.lock().await.drain(..).collect::<Vec<_>>();
        self.len.store(0, Ordering::SeqCst);

        if !purged.is_empty() {
            self.db.player_song_purge().await?;
        }

        Ok(purged)
    }

    /// Remove the item at the given position.
    pub(super) async fn remove_at(&self, n: usize) -> Result<Option<Arc<Item>>> {
        let next = {
            let mut queue = self.queue.lock().await;

            if queue.is_empty() {
                return Ok(None);
            }

            queue.remove(n)
        };

        if let Some(item) = next {
            self.len.fetch_sub(1, Ordering::SeqCst);
            self.db.player_remove_song(item.track_id(), false).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element.
    pub(super) async fn remove_last(&self) -> Result<Option<Arc<Item>>> {
        let next = {
            let mut queue = self.queue.lock().await;

            if queue.is_empty() {
                return Ok(None);
            }

            queue.pop_back()
        };

        if let Some(item) = next {
            self.len.fetch_sub(1, Ordering::SeqCst);
            self.db.player_remove_song(item.track_id(), false).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last requested song matching the given user.
    pub(super) async fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>> {
        let removed = {
            let mut queue = self.queue.lock().await;

            if queue.is_empty() {
                return Ok(None);
            }

            if let Some(position) = queue
                .iter()
                .rposition(|i| i.user().map(|u| u == user).unwrap_or_default())
            {
                queue.remove(position)
            } else {
                None
            }
        };

        if let Some(item) = removed {
            self.len.fetch_sub(1, Ordering::SeqCst);
            self.db.player_remove_song(item.track_id(), false).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Promote the given song.
    pub(super) async fn promote_song(
        &self,
        user: Option<&str>,
        n: usize,
    ) -> Result<Option<Arc<Item>>> {
        let next = {
            let mut queue = self.queue.lock().await;

            // OK, but song doesn't exist or index is out of bound.
            if queue.is_empty() || n >= queue.len() {
                return Ok(None);
            }

            if let Some(removed) = queue.remove(n) {
                queue.push_front(removed);
            }

            queue.get(0).cloned()
        };

        if let Some(item) = next {
            self.db.player_promote_song(user, item.track_id()).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Check if a song has been queued within the specified period of time.
    pub(super) async fn last_song_within(
        &self,
        track_id: &TrackId,
        duration: Duration,
    ) -> Result<Option<db::models::Song>> {
        self.db.player_last_song_within(track_id, duration).await
    }

    /// Get next song to play.
    ///
    /// Will shuffle all fallback items and add them to a queue to avoid playing the same song twice.
    pub(super) async fn next_fallback_item(&self) -> Option<Song> {
        use rand::seq::SliceRandom;

        let mut fallback = self.fallback.lock().await;

        while fallback.queue.len() < Self::FALLBACK_QUEUE_SIZE && !fallback.items.is_empty() {
            let mut rng = rand::thread_rng();
            let mut extension = fallback.items.clone();
            extension.shuffle(&mut rng);
            fallback.queue.extend(extension);
        }

        let item = fallback.queue.pop_front()?;
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
    pub(super) async fn next_song(&self) -> Result<Option<Song>> {
        if let Some(song) = self.sidelined.lock().pop_front() {
            return Ok(Some(song));
        }

        // Take next from queue.
        if let Some(item) = self.pop_front().await? {
            return Ok(Some(Song::new(item, Default::default())));
        }

        Ok(self.next_fallback_item().await)
    }

    /// Pop the front of the queue.
    async fn pop_front(&self) -> Result<Option<Arc<Item>>> {
        let next = self.queue.lock().await.pop_front();

        if let Some(item) = next {
            self.len.fetch_sub(1, Ordering::SeqCst);
            self.db.player_remove_song(item.track_id(), true).await?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    /// Push a song to the sidelined queue.
    pub(super) fn push_sidelined(&self, song: Song) {
        self.sidelined.lock().push_back(song);
    }

    /// Update available fallback items and clear the current fallback queue.
    pub(super) async fn update_fallback_items(&self, items: Vec<Arc<Item>>) {
        let mut fallback = self.fallback.lock().await;
        fallback.items = items;
        fallback.queue.clear();
    }
}
