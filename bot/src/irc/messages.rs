use std::collections::HashMap;
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::Instrument;

use crate::stream::{StreamExt, StreamMap};
use crate::Settings;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Optional {
    id: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Required {
    id: &'static str,
    default: &'static str,
}

pub(crate) const JOIN_CHAT: Optional = Optional { id: "join-chat" };
pub(crate) const LEAVE_CHAT: Optional = Optional { id: "leave-chat" };
pub(crate) const AUTH_FAILED: Required = Required {
    id: "auth-failed",
    default: "You are not allowed to run that command",
};
pub(crate) const AUTH_FAILED_RUDE: Required = Required {
    id: "auth-failed-rude",
    default: "Do you think this is a democracy? LUL",
};

const REQUIRED: [Required; 2] = [AUTH_FAILED, AUTH_FAILED_RUDE];

const OPTIONAL: [Optional; 2] = [JOIN_CHAT, LEAVE_CHAT];

/// Set up message handler.
pub(super) async fn setup(
    settings: &Settings,
) -> Result<(Messages, impl Future<Output = Result<()>>)> {
    let settings = settings.scoped("messages");

    let mut map = HashMap::new();
    let mut stream_map = StreamMap::new();

    for id in REQUIRED
        .iter()
        .map(|m| m.id)
        .chain(OPTIONAL.iter().map(|m| m.id))
    {
        let (mut stream, message) = settings.stream::<String>(id).optional().await?;

        let stream = Box::pin(async_stream::stream! {
            loop {
                yield stream.recv().await;
            }
        });

        stream_map.insert(id, stream);

        if let Some(message) = message {
            map.insert(id, message);
        }
    }

    let messages = Messages {
        map: Arc::new(RwLock::new(map)),
    };

    let messages2 = messages.clone();

    let future = async move {
        let mut stream_map = pin!(stream_map);

        while let Some((key, value)) = stream_map.next().await {
            let mut map = messages.map.write().await;

            if let Some(value) = value {
                map.insert(key, value);
            } else {
                map.remove(key);
            }
        }

        Ok(())
    };

    Ok((messages2, future.in_current_span()))
}

/// Handler for messages that can be configured.
#[derive(Default, Clone)]
pub(crate) struct Messages {
    map: Arc<RwLock<HashMap<&'static str, String>>>,
}

impl Messages {
    /// Get a message.
    pub(crate) async fn get(&self, required: Required) -> RwLockReadGuard<'_, str> {
        RwLockReadGuard::map(self.map.read().await, |map| {
            map.get(required.id)
                .map(String::as_str)
                .unwrap_or(required.default)
        })
    }

    /// Get an optional message.
    pub(crate) async fn try_get(&self, optional: Optional) -> Option<RwLockReadGuard<'_, str>> {
        RwLockReadGuard::try_map(self.map.read().await, |map| {
            map.get(optional.id).map(String::as_str)
        })
        .ok()
    }
}
