use super::Item;
use crate::{db, settings, track_id::TrackId, utils};
use anyhow::Error;
use chrono::Utc;
use std::{collections::VecDeque, sync::Arc};

/// The playback queue.
#[derive(Clone)]
pub(super) struct Queue {
    db: db::Database,
    // TODO: restrict visibility.
    pub(super) queue: settings::Var<VecDeque<Arc<Item>>>,
}

impl Queue {
    /// Construct a new queue.
    pub(super) fn new(db: db::Database) -> Self {
        Self {
            db,
            queue: settings::Var::new(Default::default()),
        }
    }

    /// Check ifa song has been queued within the specified period of time.
    pub(super) async fn last_song_within(
        &self,
        track_id: &TrackId,
        duration: utils::Duration,
    ) -> Result<Option<db::models::Song>, Error> {
        self.db.player_last_song_within(track_id, duration).await
    }

    /// Get the front of the queue.
    pub(super) async fn front(&self) -> Option<Arc<Item>> {
        self.queue.read().await.front().cloned()
    }

    /// Pop the front of the queue.
    pub(super) async fn pop_front(&self) -> Result<Option<Arc<Item>>, Error> {
        let db = self.db.clone();
        // NB: hold the lock over the database modification.
        let mut queue = self.queue.write().await;

        if let Some(item) = queue.pop_front() {
            db.player_remove_song(&item.track_id).await?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    /// Push item to back of queue.
    pub(super) async fn push_back(&self, item: Arc<Item>) -> Result<(), Error> {
        // NB: hold the lock over the database modification.
        let mut queue = self.queue.write().await;

        self.db
            .player_push_back(&db::models::AddSong {
                track_id: item.track_id.clone(),
                added_at: Utc::now().naive_utc(),
                user: item.user.clone(),
            })
            .await?;

        queue.push_back(item);
        Ok(())
    }

    /// Purge the song queue.
    pub(super) async fn purge(&self) -> Result<Vec<Arc<Item>>, Error> {
        let mut q = self.queue.write().await;

        if q.is_empty() {
            return Ok(vec![]);
        }

        let purged = std::mem::replace(&mut *q, VecDeque::new())
            .into_iter()
            .collect();

        self.db.player_song_purge().await?;
        Ok(purged)
    }

    /// Remove the item at the given position.
    pub(super) async fn remove_at(&self, n: usize) -> Result<Option<Arc<Item>>, Error> {
        let mut q = self.queue.write().await;

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(item) = q.remove(n) {
            self.db.player_remove_song(&item.track_id).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element.
    pub(super) async fn remove_last(&self) -> Result<Option<Arc<Item>>, Error> {
        let mut q = self.queue.write().await;

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(item) = q.pop_back() {
            self.db.player_remove_song(&item.track_id).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Remove the last element by user.
    pub(super) async fn remove_last_by_user(&self, user: &str) -> Result<Option<Arc<Item>>, Error> {
        let mut q = self.queue.write().await;

        if q.is_empty() {
            return Ok(None);
        }

        if let Some(position) = q
            .iter()
            .rposition(|i| i.user.as_ref().map(|u| u == user).unwrap_or_default())
        {
            if let Some(item) = q.remove(position) {
                self.db.player_remove_song(&item.track_id).await?;
                return Ok(Some(item));
            }
        }

        Ok(None)
    }

    /// Promote the given song.
    pub(super) async fn promote_song(
        &self,
        user: Option<&str>,
        n: usize,
    ) -> Result<Option<Arc<Item>>, Error> {
        let mut q = self.queue.write().await;

        // OK, but song doesn't exist or index is out of bound.
        if q.is_empty() || n >= q.len() {
            return Ok(None);
        }

        if let Some(removed) = q.remove(n) {
            q.push_front(removed);
        }

        if let Some(item) = q.get(0).cloned() {
            self.db.player_promote_song(user, &item.track_id).await?;
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Push item to back of queue without going through the database.
    pub(super) async fn push_back_queue(&self, item: Arc<Item>) {
        self.queue.write().await.push_back(item);
    }
}
