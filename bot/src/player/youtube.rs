use crate::{bus, player};
use futures::{sync, Async, Poll, Stream};
use std::sync::Arc;
use tokio::timer;

/// Setup a player.
pub fn setup(bus: Arc<bus::Bus>) -> Result<YouTubePlayer, failure::Error> {
    let (tx, rx) = sync::mpsc::unbounded();
    Ok(YouTubePlayer {
        bus,
        tx,
        rx,
        timeout: None,
    })
}

pub struct YouTubePlayer {
    bus: Arc<bus::Bus>,
    tx: sync::mpsc::UnboundedSender<player::IntegrationEvent>,
    rx: sync::mpsc::UnboundedReceiver<player::IntegrationEvent>,
    /// Timeout for end of song.
    timeout: Option<timer::Delay>,
}

impl YouTubePlayer {
    /// Update playback information.
    pub fn tick(&mut self, song: &player::Song, video_id: &str) {
        let event = bus::YouTubeEvent::Play {
            video_id: video_id.to_string(),
            elapsed: song.elapsed().as_secs(),
            duration: song.duration().as_secs(),
        };

        self.bus.send(bus::Message::YouTubeCurrent { event });
    }

    /// Detach the player, cancelling any timed events or effects.
    pub fn detach(&mut self) {
        self.timeout = None;
    }

    pub fn play(&mut self, source: super::Source, song: &player::Song, video_id: &str) {
        let event = bus::YouTubeEvent::Play {
            video_id: video_id.to_string(),
            elapsed: song.elapsed().as_secs(),
            duration: song.duration().as_secs(),
        };

        self.bus.send(bus::Message::YouTubeCurrent { event });
        self.timeout = Some(timer::Delay::new(song.deadline()));
        self.send(player::IntegrationEvent::Playing(source));
    }

    pub fn pause(&mut self, source: super::Source) {
        let event = bus::YouTubeEvent::Pause;
        self.bus.send(bus::Message::YouTubeCurrent { event });
        self.timeout = None;
        self.send(player::IntegrationEvent::Pausing(source));
    }

    pub fn stop(&mut self) {
        let event = bus::YouTubeEvent::Stop;
        self.bus.send(bus::Message::YouTubeCurrent { event });
        self.timeout = None;
        self.send(player::IntegrationEvent::Stopping);
    }

    pub fn volume(&mut self, source: super::Source, volume: u32) {
        self.bus.send(bus::Message::YouTubeVolume { volume });
        self.send(player::IntegrationEvent::Volume(source, volume));
    }

    /// Send an integration event.
    fn send(&mut self, event: player::IntegrationEvent) {
        if let Err(_) = self.tx.unbounded_send(event) {
            log::error!("failed to send integration event");
        }
    }
}

impl Stream for YouTubePlayer {
    type Item = player::IntegrationEvent;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, failure::Error> {
        use futures::Future as _;

        if let Some(timeout) = self.timeout.as_mut() {
            if let Async::Ready(()) = timeout.poll()? {
                self.timeout = None;
                return Ok(Async::Ready(Some(player::IntegrationEvent::EndOfTrack)));
            }
        }

        if let Some(e) = try_infinite_empty!(self.rx.poll()) {
            return Ok(Async::Ready(Some(e)));
        }

        Ok(Async::NotReady)
    }
}
