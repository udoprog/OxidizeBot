use anyhow::{anyhow, Error};
use ring::aead;
use ring::pbkdf2;
use ring::rand::{SecureRandom as _, SystemRandom};
use std::num;

/// A helper type to seal and unseal messages using AEAD.
///
/// A sealed message is both encrypted and signed in one go.
pub struct AeadSealer {
    random: SystemRandom,
    alg: &'static aead::Algorithm,
    key: aead::LessSafeKey,
}

impl AeadSealer {
    /// Create a new sealer from a secret which isn't necessarily as long as the expected key.
    pub fn from_secret(alg: &'static aead::Algorithm, secret: &[u8]) -> Result<AeadSealer, Error> {
        // Keys are sent as &[T] and must have 32 bytes
        let mut key = vec![0u8; alg.key_len()];

        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            num::NonZeroU32::new(100).unwrap(),
            &[],
            secret,
            &mut key,
        );

        Self::new(alg, &key)
    }

    /// Create a new store with the specified secret key.
    fn new(alg: &'static aead::Algorithm, key: &[u8]) -> Result<AeadSealer, Error> {
        let random = SystemRandom::new();
        Self::inner(random, alg, key)
    }

    /// Create a new store with a random key.
    pub fn random(alg: &'static aead::Algorithm) -> Result<AeadSealer, Error> {
        let random = SystemRandom::new();
        let mut key = vec![0u8; alg.key_len()];

        random
            .fill(&mut key)
            .map_err(|_| anyhow!("failed to fill random key"))?;

        AeadSealer::inner(random, alg, &key)
    }

    /// Inner constructor.
    fn inner(
        random: SystemRandom,
        alg: &'static aead::Algorithm,
        key: &[u8],
    ) -> Result<AeadSealer, Error> {
        let key = aead::UnboundKey::new(alg, key).map_err(|_| anyhow!("failed to create key"))?;

        Ok(AeadSealer {
            random,
            alg,
            key: aead::LessSafeKey::new(key),
        })
    }

    /// Encrypt the given message.
    pub fn encrypt(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        let mut nonce_buf = [0u8; 12];
        self.random
            .fill(&mut nonce_buf)
            .map_err(|_| anyhow!("failed to fill random nonce"))?;
        let nonce = aead::Nonce::assume_unique_for_key(nonce_buf);
        let aad = aead::Aad::empty();

        let mut ciphertext =
            Vec::with_capacity(nonce_buf.len() + message.len() + self.key.algorithm().tag_len());
        ciphertext.extend(&nonce_buf);
        ciphertext.extend(message);

        let tag = self
            .key
            .seal_in_place_separate_tag(nonce, aad, &mut ciphertext[nonce_buf.len()..])
            .map_err(|_| anyhow!("failed to seal data"))?;

        ciphertext.extend(tag.as_ref());
        Ok(ciphertext)
    }

    /// Decrypt the given ciphertext.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let nonce_len = self.alg.nonce_len();
        let tag_len = self.alg.tag_len();

        if ciphertext.len() < nonce_len + tag_len {
            return Ok(None);
        }

        let nonce = aead::Nonce::try_assume_unique_for_key(&ciphertext[0..nonce_len])
            .map_err(|_| anyhow!("failed to extract nonce"))?;

        let aad = aead::Aad::empty();

        let mut out = Vec::new();
        out.extend_from_slice(&ciphertext[nonce_len..]);

        let plain_len = match self.key.open_in_place(nonce, aad, &mut out).ok() {
            Some(out) => out.len(),
            None => return Ok(None),
        };

        out.truncate(plain_len);
        Ok(Some(out))
    }
}
