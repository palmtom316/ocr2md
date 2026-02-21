use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

use ocr2md_core::{
    profile_store::{ProfileStore, ProviderProfile},
    queue::Queue,
};

#[derive(Clone)]
pub struct AppState {
    pub queue: Arc<Mutex<Queue>>,
    profile_store: ProfileStore,
    pub notify_worker: Arc<Notify>,
    pub active_profiles: Arc<Mutex<Vec<ProviderProfile>>>,
}

impl AppState {
    pub fn for_profile_path(path: PathBuf) -> Self {
        Self {
            queue: Arc::new(Mutex::new(Queue::default())),
            profile_store: ProfileStore::new(path),
            notify_worker: Arc::new(Notify::new()),
            active_profiles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn profile_store(&self) -> &ProfileStore {
        &self.profile_store
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::for_profile_path(default_profile_path())
    }
}

fn default_profile_path() -> PathBuf {
    if let Ok(explicit_path) = std::env::var("OCR2MD_PROFILE_STORE_PATH") {
        let trimmed = explicit_path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    let mut root = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    root.push("ocr2md-desktop");
    root.push("profiles.enc");
    root
}
