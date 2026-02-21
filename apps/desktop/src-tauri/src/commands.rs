use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;
use ocr2md_core::profile_store::ProviderProfile;

pub fn enqueue_files_inner(state: &AppState, files: Vec<String>) -> Vec<u64> {
    let mut queue = state.queue.lock().expect("queue mutex poisoned");
    let ids: Vec<u64> = files.into_iter().map(|file| queue.enqueue(file)).collect();
    state.notify_worker.notify_one();
    ids
}

#[tauri::command]
pub fn enqueue_files(files: Vec<String>, state: State<'_, AppState>) -> Result<Vec<u64>, String> {
    Ok(enqueue_files_inner(&state, files))
}

#[tauri::command]
pub fn start_queue(state: State<'_, AppState>) -> Result<(), String> {
    state.notify_worker.notify_one();
    Ok(())
}

#[tauri::command]
pub fn retry_job(id: u64, state: State<'_, AppState>) -> Result<(), String> {
    let mut queue = state.queue.lock().expect("queue mutex poisoned");
    queue.mark_running(id, "retry");
    state.notify_worker.notify_one();
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfilePayload {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub enabled: bool,
}

impl From<ProviderProfile> for ProviderProfilePayload {
    fn from(value: ProviderProfile) -> Self {
        Self {
            name: value.name,
            provider: value.provider,
            base_url: value.base_url,
            api_key: value.api_key,
            model: value.model,
            enabled: value.enabled,
        }
    }
}

impl From<ProviderProfilePayload> for ProviderProfile {
    fn from(value: ProviderProfilePayload) -> Self {
        Self {
            name: value.name,
            provider: value.provider,
            base_url: value.base_url,
            api_key: value.api_key,
            model: value.model,
            enabled: value.enabled,
        }
    }
}

fn normalize_passphrase(passphrase: &str) -> Result<&str, String> {
    let trimmed = passphrase.trim();
    if trimmed.is_empty() {
        return Err("passphrase must not be empty".to_string());
    }
    Ok(trimmed)
}

pub fn load_profiles_inner(
    state: &AppState,
    passphrase: &str,
) -> Result<Vec<ProviderProfilePayload>, String> {
    let passphrase = normalize_passphrase(passphrase)?;
    let profiles = state
        .profile_store()
        .load_all(passphrase)
        .map_err(|error| format!("failed to load profiles: {error}"))?;

    *state.active_profiles.lock().unwrap() = profiles.clone();

    Ok(profiles
        .into_iter()
        .map(ProviderProfilePayload::from)
        .collect())
}

pub fn save_profiles_inner(
    state: &AppState,
    passphrase: &str,
    profiles: Vec<ProviderProfilePayload>,
) -> Result<(), String> {
    let passphrase = normalize_passphrase(passphrase)?;
    let mapped: Vec<ProviderProfile> = profiles.into_iter().map(ProviderProfile::from).collect();
    state
        .profile_store()
        .save_all(passphrase, &mapped)
        .map_err(|error| format!("failed to save profiles: {error}"))?;

    *state.active_profiles.lock().unwrap() = mapped;
    Ok(())
}

#[tauri::command]
pub fn load_profiles(
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProviderProfilePayload>, String> {
    load_profiles_inner(&state, &passphrase)
}

#[tauri::command]
pub fn save_profiles(
    passphrase: String,
    profiles: Vec<ProviderProfilePayload>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    save_profiles_inner(&state, &passphrase, profiles)
}
