use std::pin::pin;
use std::sync::Arc;
use std::{borrow::Cow, fmt};

use serde::{de, ser, Deserialize, Serialize};
use tokio::sync::Notify;

/// Security wrapper for a token. This reduces the risk that the token is
/// inadvertently printed as it as a redacted debug implementation.
pub struct TokenPayload(Arc<str>);

impl TokenPayload {
    /// Get the string of the token.
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl fmt::Debug for TokenPayload {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TokenPayload").field(&"*secret*").finish()
    }
}

impl Serialize for TokenPayload {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(self.0.as_ref())
    }
}

impl<'de> Deserialize<'de> for TokenPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        Ok(Self(Arc::from(
            Cow::<str>::deserialize(deserializer)?.as_ref(),
        )))
    }
}

/// A synchronized token holder for which we can asynchronously process events
/// in multiple directions.
#[derive(Default, Clone)]
pub struct Token {
    inner: Arc<Inner>,
}

impl Token {
    /// Construct a new empty token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the token.
    pub fn clear(&self) {
        *self.inner.payload.write() = None;
    }

    /// Read the current token.
    pub fn read(&self) -> Option<(TokenPayload, Arc<str>)> {
        let payload = self.inner.payload.read();
        let payload = payload.as_ref()?;

        Some((
            TokenPayload(payload.token.clone()),
            payload.client_id.clone(),
        ))
    }

    /// Clear the current token, and indicate that it needs to be refreshed.
    pub fn force_refresh(&self) {
        *self.inner.payload.write() = None;
        self.inner.refresh.notify_one();
    }

    /// Wait for the token to need to be refreshed.
    ///
    /// Only a single waker is notified *once* if a refresh is needed.
    pub async fn wait_for_refresh(&self) {
        self.inner.refresh.notified().await;
    }

    /// Test if token is ready.
    pub fn is_ready(&self) -> bool {
        self.inner.payload.read().is_some()
    }

    /// Sets the value of the token, notifying anyone waiting for it to be
    /// set.
    pub fn set(&self, token: &str, client_id: &str) {
        *self.inner.payload.write() = Some(Payload {
            token: token.into(),
            client_id: client_id.into(),
        });

        self.inner.waiters.notify_waiters();
    }

    /// Wait until token is ready.
    pub async fn wait_until_ready(&self) {
        let mut future = pin!(self.inner.waiters.notified());

        loop {
            future.as_mut().enable();

            if self.inner.payload.read().is_some() {
                break;
            }

            future.as_mut().await;
            future.set(self.inner.waiters.notified());
        }
    }
}

struct Payload {
    token: Arc<str>,
    client_id: Arc<str>,
}

#[derive(Default)]
struct Inner {
    payload: parking_lot::RwLock<Option<Payload>>,
    refresh: Notify,
    waiters: Notify,
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Token");

        let payload = self.inner.payload.read();

        if payload.as_ref().map(|p| p.token.as_ref()).is_some() {
            f.field("token", &"*secret*");
        } else {
            f.field("token", &"none");
        }

        if let Some(client_id) = payload.as_ref().map(|p| p.client_id.as_ref()) {
            f.field("client_id", &client_id);
        } else {
            f.field("client_id", &"none");
        }

        f.finish()
    }
}
