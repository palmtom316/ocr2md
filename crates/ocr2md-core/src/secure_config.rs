use anyhow::{Result, anyhow, bail};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;

const MAGIC: [u8; 4] = *b"O2MD";
const VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const TAG_LEN: usize = 16;

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    if passphrase.is_empty() {
        bail!("passphrase cannot be empty");
    }

    let params = Params::new(19_456, 2, 1, Some(KEY_LEN))
        .map_err(|err| anyhow!("failed to initialize argon2 params: {err}"))?;
    let mut key = [0_u8; KEY_LEN];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|err| anyhow!("failed to derive encryption key: {err}"))?;
    Ok(key)
}

pub fn encrypt_blob(plain: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let mut salt = [0_u8; SALT_LEN];
    let mut nonce = [0_u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    rand::rngs::OsRng.fill_bytes(&mut nonce);

    let key = derive_key(passphrase, &salt)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plain)
        .map_err(|_| anyhow!("failed to encrypt blob"))?;

    let mut out = Vec::with_capacity(MAGIC.len() + 1 + SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_blob(blob: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let min_len = MAGIC.len() + 1 + SALT_LEN + NONCE_LEN + TAG_LEN;
    if blob.len() < min_len {
        bail!("ciphertext envelope is too short");
    }

    let (magic, rest) = blob.split_at(MAGIC.len());
    if magic != MAGIC {
        bail!("unsupported ciphertext envelope");
    }

    let (&version, rest) = rest
        .split_first()
        .ok_or_else(|| anyhow!("missing ciphertext version"))?;
    if version != VERSION {
        bail!("unsupported ciphertext version: {version}");
    }

    let (salt, rest) = rest.split_at(SALT_LEN);
    let (nonce, ciphertext) = rest.split_at(NONCE_LEN);

    let key = derive_key(passphrase, salt)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let plain = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| anyhow!("failed to decrypt blob"))?;
    Ok(plain)
}
