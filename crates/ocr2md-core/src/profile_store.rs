use crate::secure_config::{decrypt_blob, encrypt_blob};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const STORE_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfile {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl ProviderProfile {
    pub fn openai(name: &str, base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            name: name.to_string(),
            provider: "openai".to_string(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileStore {
    path: PathBuf,
}

impl ProfileStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn save_all(&self, passphrase: &str, profiles: &[ProviderProfile]) -> Result<()> {
        let payload = StoreEnvelope {
            version: STORE_VERSION,
            profiles: profiles.to_vec(),
        };
        let plain = serde_json::to_vec(&payload).context("failed to serialize profiles")?;
        let ciphertext = encrypt_blob(&plain, passphrase).context("failed to encrypt profiles")?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).context("failed to create profile directory")?;
        }
        fs::write(&self.path, ciphertext).context("failed to write encrypted profile store")?;
        Ok(())
    }

    pub fn load_all(&self, passphrase: &str) -> Result<Vec<ProviderProfile>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let ciphertext = fs::read(&self.path).context("failed to read encrypted profile store")?;
        let plain = decrypt_blob(&ciphertext, passphrase).context("failed to decrypt profiles")?;
        let payload: StoreEnvelope =
            serde_json::from_slice(&plain).context("failed to deserialize profiles")?;
        Ok(payload.profiles)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoreEnvelope {
    #[serde(default = "default_store_version")]
    version: u8,
    #[serde(default)]
    profiles: Vec<ProviderProfile>,
}

fn default_store_version() -> u8 {
    STORE_VERSION
}
