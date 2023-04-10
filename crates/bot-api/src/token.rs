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
#[derive(Clone)]
pub struct Token {
    inner: Arc<Inner>,
}

impl Token {
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

    /// Test if token is ready.
    pub fn is_ready(&self) -> bool {
        self.inner.payload.read().is_some()
    }

    /// Wait until token is ready.
    pub async fn wait_until_read(&self) {
        todo!()
    }
}

struct Payload {
    token: Arc<str>,
    client_id: Arc<str>,
}

struct Inner {
    payload: parking_lot::RwLock<Option<Payload>>,
    refresh: Notify,
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Token");

        let payload = self.inner.payload.read();

        if let Some(..) = payload.as_ref().map(|p| p.token.as_ref()) {
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
