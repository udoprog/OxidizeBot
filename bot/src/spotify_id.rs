// Copied and modified from the librespot project, the following is their license:
//
// The MIT License (MIT)
//
// Copyright (c) 2015 Paul Lietar
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use std::fmt;
use std::str;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpotifyId(u128);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SpotifyIdError;

impl fmt::Display for SpotifyIdError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "bad spotify id")
    }
}

const BASE62_DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

impl SpotifyId {
    pub fn from_base62(id: &str) -> Result<SpotifyId, SpotifyIdError> {
        let data = id.as_bytes();

        let mut n = 0u128;

        for c in data {
            let d = match BASE62_DIGITS.iter().position(|e| e == c) {
                None => return Err(SpotifyIdError),
                Some(x) => x as u128,
            };
            n *= 62u128;
            n += d;
        }

        Ok(SpotifyId(n))
    }

    pub fn to_base62(&self) -> String {
        let &SpotifyId(mut n) = self;

        let mut data = [0u8; 22];
        let sixty_two = 62u128;
        for i in 0..22 {
            data[21 - i] = BASE62_DIGITS[(n % sixty_two) as u64 as usize];
            n /= sixty_two;
        }

        str::from_utf8(&data).unwrap().to_owned()
    }
}

impl fmt::Display for SpotifyId {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.to_base62())
    }
}

impl serde::Serialize for SpotifyId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_base62().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for SpotifyId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SpotifyId::from_base62(&s).map_err(serde::de::Error::custom)
    }
}
