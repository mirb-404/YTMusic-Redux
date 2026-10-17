//! Runtime-owned persisted data that the settings UI never writes directly.

use crate::settings::write_json_atomic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthToken {
    pub app_id: String,
    pub app_name: String,
    pub app_version: String,
    pub id: String,
    /// SHA-256 of the issued token; the raw token is only ever sent to the client once.
    pub token_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppState {
    pub last_url: String,
    pub last_playlist_id: String,
    pub last_video_id: String,
    pub window_bounds: Option<WindowBounds>,
    pub window_maximized: bool,
    pub companion_tokens: Vec<AuthToken>,
    /// Pending Last.fm auth token awaiting user approval in the browser.
    pub lastfm_token: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            last_url: crate::ytm::YTM_HOME.into(),
            last_playlist_id: String::new(),
            last_video_id: String::new(),
            window_bounds: None,
            window_maximized: false,
            companion_tokens: Vec::new(),
            lastfm_token: None,
        }
    }
}

pub struct StateStore {
    path: PathBuf,
    state: Mutex<AppState>,
}

impl StateStore {
    pub fn load(path: PathBuf) -> Self {
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self { path, state: Mutex::new(state) }
    }

    pub fn read<T>(&self, f: impl FnOnce(&AppState) -> T) -> T {
        f(&self.state.lock().unwrap())
    }

    /// Mutates in memory only (for high-frequency fields like the last URL).
    pub fn update_volatile(&self, f: impl FnOnce(&mut AppState)) {
        f(&mut self.state.lock().unwrap());
    }

    pub fn update(&self, f: impl FnOnce(&mut AppState)) {
        let mut guard = self.state.lock().unwrap();
        f(&mut guard);
        if let Err(e) = write_json_atomic(&self.path, &*guard) {
            log::error!("Failed to save app state: {e}");
        }
    }

    pub fn save(&self) {
        self.update(|_| {});
    }
}

/// OS credential store for secrets (upstream used Electron safeStorage).
pub mod secrets {
    const SERVICE: &str = "ytmusic-redux";

    pub fn get(name: &str) -> Option<String> {
        keyring::Entry::new(SERVICE, name).ok()?.get_password().ok()
    }

    pub fn set(name: &str, value: Option<&str>) {
        let Ok(entry) = keyring::Entry::new(SERVICE, name) else { return };
        let result = match value {
            Some(v) => entry.set_password(v),
            None => match entry.delete_credential() {
                Err(keyring::Error::NoEntry) => Ok(()),
                other => other,
            },
        };
        if let Err(e) = result {
            log::error!("Credential store write failed for {name}: {e}");
        }
    }

    pub fn available() -> bool {
        keyring::Entry::new(SERVICE, "probe").is_ok()
    }
}
