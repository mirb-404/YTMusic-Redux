//! In-memory runtime flags shared with the UI windows (port of upstream `MemoryStoreSchema`).

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub discord_presence_connection_failed: bool,
    /// Shortcut setting key -> registration failed.
    pub shortcut_register_failed: BTreeMap<String, bool>,
    pub companion_server_auth_window_enabled: bool,
    pub companion_server_error: Option<String>,
    pub lastfm_status: Option<String>,
    pub secure_storage_available: bool,
    pub ytm_view_loading: bool,
    pub ytm_view_load_timedout: bool,
    pub ytm_view_loading_status: String,
    pub restart_required: bool,
    /// Unix time in milliseconds when the sleep timer pauses playback.
    pub sleep_timer_ends_at: Option<u64>,
    /// The app's now playing (lyrics) view covers the YTM view.
    pub now_playing_open: bool,
    /// The bridge drew the window controls into YTM's header, so the app title bar is hidden.
    pub ytm_owns_header: bool,
    /// The first-start tour of the app's controls covers the YTM view.
    pub tour_open: bool,
    /// YTM's player page (video, queue) is open; with native browsing it is the only time
    /// the YTM page is shown.
    pub ytm_player_page: bool,
    /// The YTM view shows a page that is not YouTube Music (sign-in, consent), which needs
    /// the user.
    pub ytm_foreign_page: bool,
}

pub struct MemoryStore {
    inner: Mutex<Memory>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self { inner: Mutex::new(Memory::default()) }
    }

    pub fn get(&self) -> Memory {
        self.inner.lock().unwrap().clone()
    }

    pub fn read<T>(&self, f: impl FnOnce(&Memory) -> T) -> T {
        f(&self.inner.lock().unwrap())
    }

    /// Applies a change and notifies every window.
    pub fn update(&self, app: &AppHandle, f: impl FnOnce(&mut Memory)) {
        let snapshot = {
            let mut guard = self.inner.lock().unwrap();
            f(&mut guard);
            guard.clone()
        };
        let _ = app.emit("memory-changed", snapshot);
    }
}
