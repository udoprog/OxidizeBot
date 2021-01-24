use crate::bus;
use crate::emotes;
use crate::irc;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// Indicates if the system is enabled or not.
    #[serde(rename = "enabled")]
    Enabled { enabled: bool },
    /// Indicate that the given message has been received.
    #[serde(rename = "message")]
    Message(Message),
    /// Indicates that a message with the given ID has been deleted.
    #[serde(rename = "delete-by-id")]
    DeleteById { id: String },
    /// Indicates that all messages by the given user have been deleted.
    #[serde(rename = "delete-by-user")]
    DeleteByUser { name: String },
    /// Delete all messages.
    #[serde(rename = "delete-all")]
    DeleteAll,
}

impl bus::Message for Event {
    /// The ID of a bussed message.
    fn id(&self) -> Option<&'static str> {
        match *self {
            Event::Enabled { .. } => Some("enabled"),
            _ => None,
        }
    }
}

/// A builder for MessageLog.
#[derive(Default)]
pub struct Builder {
    limit: Option<usize>,
    bus: Option<bus::Bus<Event>>,
}

impl Builder {
    /// How many messages to store.
    pub fn limit(self, limit: usize) -> Self {
        Self {
            limit: Some(limit),
            ..self
        }
    }

    /// Associate a bus with the log.
    pub fn bus(self, bus: bus::Bus<Event>) -> Self {
        Self {
            bus: Some(bus),
            ..self
        }
    }

    /// Construct a new message log.
    pub fn build(self) -> MessageLog {
        MessageLog {
            inner: Arc::new(RwLock::new(Inner {
                enabled: true,
                limit: self.limit,
                bus: self.bus,
                messages: Default::default(),
            })),
        }
    }
}

pub struct Inner {
    enabled: bool,
    limit: Option<usize>,
    bus: Option<bus::Bus<Event>>,
    messages: VecDeque<Message>,
}

/// In-memory log of commands.
#[derive(Clone)]
pub struct MessageLog {
    inner: Arc<RwLock<Inner>>,
}

impl MessageLog {
    /// Get a new builder for a message log.
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Get a copy of all the messages.
    pub async fn messages(&self) -> RwLockReadGuard<'_, VecDeque<Message>> {
        RwLockReadGuard::map(self.inner.read().await, |i| &i.messages)
    }

    /// Indicate if the log is enabled or not.
    pub async fn enabled(&self, enabled: bool) {
        if let Some(bus) = self.inner.read().await.bus.as_ref() {
            bus.send(Event::Enabled { enabled }).await;
        }
    }

    /// Mark the given message as deleted.
    pub async fn delete_by_id(&self, id: &str) {
        let mut inner = self.inner.write().await;

        for m in &mut inner.messages {
            if m.id == id {
                m.deleted = true;
            }
        }

        if let Some(bus) = inner.bus.as_ref() {
            bus.send(Event::DeleteById { id: id.to_string() }).await;
        }
    }

    /// Mark all messages by the given user as deleted.
    pub async fn delete_by_user(&self, name: &str) {
        let mut inner = self.inner.write().await;

        for m in &mut inner.messages {
            if m.user.name == name {
                m.deleted = true;
            }
        }

        if let Some(bus) = inner.bus.as_ref() {
            bus.send(Event::DeleteByUser {
                name: name.to_string(),
            })
            .await;
        }
    }

    /// Delete all messages in chat.
    pub async fn delete_all(&self) {
        let mut inner = self.inner.write().await;

        for m in &mut inner.messages {
            m.deleted = true;
        }

        if let Some(bus) = inner.bus.as_ref() {
            bus.send(Event::DeleteAll).await;
        }
    }

    /// Push a message to the back of the log.
    pub async fn push_back(
        &self,
        tags: &irc::Tags,
        name: &str,
        text: &str,
        rendered: Option<emotes::Rendered>,
    ) {
        let mut inner = self.inner.write().await;

        if !inner.enabled {
            return;
        }

        if let Some(limit) = inner.limit {
            while inner.messages.len() >= limit {
                inner.messages.pop_front();
            }
        }

        let id = match tags.id.as_ref() {
            Some(id) => id,
            None => return,
        };

        let user_id = match tags.user_id.as_ref() {
            Some(user_id) => user_id,
            None => return,
        };

        let display_name = match tags.display_name.as_ref() {
            Some(display_name) => display_name,
            None => return,
        };

        let user = User {
            user_id: user_id.to_string(),
            name: name.to_string(),
            display_name: display_name.to_string(),
            color: tags.color.clone(),
        };

        let m = Message {
            timestamp: Utc::now(),
            id: id.to_string(),
            user,
            text: text.to_string(),
            rendered,
            deleted: false,
        };

        if let Some(bus) = inner.bus.as_ref() {
            bus.send(Event::Message(m.clone())).await;
        }

        inner.messages.push_back(m);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    user_id: String,
    name: String,
    display_name: String,
    color: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    timestamp: DateTime<Utc>,
    id: String,
    user: User,
    text: String,
    rendered: Option<emotes::Rendered>,
    deleted: bool,
}
