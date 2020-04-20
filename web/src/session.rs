use crate::{aead::AeadSealer, db};
use anyhow::Error;
use cookie::{Cookie, CookieBuilder, CookieJar};
use hyper::{header, HeaderMap, Request};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    secret: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SessionData {
    /// Twitch user id.
    pub user_id: String,
}

pub struct Session {
    db: db::Database,
    sealer: AeadSealer,
}

impl Session {
    pub fn new(db: db::Database, config: &Config) -> Result<Self, Error> {
        let sealer = match config.secret.as_ref() {
            Some(secret) => AeadSealer::from_secret(&ring::aead::AES_256_GCM, secret.as_bytes())?,
            None => AeadSealer::random(&ring::aead::AES_256_GCM)?,
        };

        Ok(Self { db, sealer })
    }

    /// Set the given cookie.
    pub fn set_cookie<T>(&self, headers: &mut HeaderMap, name: &str, value: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let data = serde_cbor::to_vec(&value)?;
        let data = self.sealer.encrypt(&data)?;
        let data = base64::encode(&data);

        let mut jar = CookieJar::new();
        jar.add(self.cookie(name.to_string(), data).finish());

        for c in jar.delta() {
            headers.insert(header::SET_COOKIE, c.encoded().to_string().parse()?);
        }

        Ok(())
    }

    /// Delete a cookie from the given request and add the appropriate Set-Cookie to another set of headers.
    pub fn delete_cookie(&self, headers: &mut HeaderMap, name: &str) -> Result<(), Error> {
        headers.insert(
            header::SET_COOKIE,
            self.cookie(name.to_string(), "")
                .expires(time::OffsetDateTime::unix_epoch())
                .finish()
                .encoded()
                .to_string()
                .parse()?,
        );

        Ok(())
    }

    /// Get cookies from the specified headers.
    pub fn cookies_from_header<B>(&self, req: &Request<B>) -> Result<Option<CookieJar>, Error> {
        let value = match req.headers().get(header::COOKIE) {
            Some(value) => value,
            None => return Ok(None),
        };

        Ok(Some(cookiejar_from_header(value.as_bytes())?))
    }

    /// Verify the given authorization header.
    fn verify_authorization_header(&self, header: &str) -> Result<Option<db::User>, Error> {
        let mut it = header.split(':');

        match it.next() {
            Some("key") => (),
            _ => return Ok(None),
        };

        let key = match it.next() {
            Some(key) => key,
            None => return Ok(None),
        };

        Ok(self.db.get_user_by_key(key)?)
    }

    /// Verify cookie.
    fn verify_cookie<B>(&self, req: &Request<B>) -> Result<Option<db::User>, Error> {
        // Get it through cookie.
        let jar = match self.cookies_from_header(req)? {
            Some(jar) => jar,
            None => return Ok(None),
        };

        let cookie = match jar.get("session") {
            Some(cookie) => cookie,
            None => return Ok(None),
        };

        let data = base64::decode(cookie.value())?;

        let data = match self.sealer.decrypt(&data)? {
            Some(data) => data,
            None => return Ok(None),
        };

        let session = serde_cbor::from_slice::<SessionData>(&data)?;
        Ok(self.db.get_user(&session.user_id)?)
    }

    /// Verify the given request and return user information (if present).
    pub fn verify<B>(&self, req: &Request<B>) -> Result<Option<db::User>, Error> {
        if let Some(authorization) = req.headers().get(header::AUTHORIZATION) {
            if let Some(user) = self.verify_authorization_header(authorization.to_str()?)? {
                return Ok(Some(user));
            }
        }

        self.verify_cookie(req)
    }

    /// Build a new cookie.
    fn cookie(
        &self,
        name: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> CookieBuilder<'static> {
        Cookie::build(name, value).http_only(true).path("/")
    }
}

/// Parse a CookieJar from a header.
fn cookiejar_from_header(header: &[u8]) -> Result<CookieJar, Error> {
    let mut jar = CookieJar::new();

    for p in header.split(|b| *b == b';') {
        let p = std::str::from_utf8(p)?;
        jar.add_original(Cookie::parse_encoded(p.trim().to_owned())?);
    }

    Ok(jar)
}
