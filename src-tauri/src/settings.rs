//! Persistent user settings (port of upstream `StoreSchema`).
//!
//! Settings are edited by key path (`"general.startOnBoot"`) so concurrent writers never
//! overwrite each other's sections with stale copies. Runtime-owned data (window state,
//! companion tokens, Last.fm session) lives in separate stores, see `state_store.rs`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TrayIconStyle {
    #[default]
    Auto,
    White,
    Black,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct General {
    pub disable_hardware_acceleration: bool,
    pub hide_to_tray_on_close: bool,
    pub show_notification_on_song_change: bool,
    pub start_on_boot: bool,
    pub start_minimized: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    /// App-rendered player bar with wave seek line plus the restyled YTM theme.
    pub modern_ui: bool,
    /// The player bar is tinted by the playing track's artwork; off is the plain dark bar.
    pub tinted_player_bar: bool,
    /// The app draws YouTube Music's pages itself (with Modern UI); the YTM page only plays
    /// and shows its player page. Off shows YTM's own pages.
    pub native_browse: bool,
    /// Leaving the video view shrinks it into a mini player instead of closing it.
    pub video_mini_player: bool,
    pub always_show_volume_slider: bool,
    pub custom_css_enabled: bool,
    pub custom_css_path: Option<String>,
    pub zoom: u32,
    pub tray_icon_style: TrayIconStyle,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            modern_ui: true,
            tinted_player_bar: true,
            native_browse: true,
            video_mini_player: false,
            always_show_volume_slider: false,
            custom_css_enabled: false,
            custom_css_path: None,
            zoom: 100,
            tray_icon_style: TrayIconStyle::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Playback {
    pub continue_where_you_left_off: bool,
    pub continue_where_you_left_off_paused: bool,
    pub enable_speaker_fill: bool,
    pub progress_in_taskbar: bool,
    pub ratio_volume: bool,
    /// Plays music videos where they exist; off plays the audio-only song version and
    /// shows the artwork instead.
    pub show_videos: bool,
    /// Playing a song from the app's pages opens the video view (video or artwork); off
    /// keeps playback in the player bar.
    pub open_player_on_play: bool,
    /// Playback speed in percent.
    pub playback_rate: u32,
    /// Seconds to fade out the end of a track and fade in the next; 0 turns it off.
    pub crossfade_seconds: u32,
    pub equalizer: Equalizer,
    /// Removes ads from YTM's responses and skips any that still play.
    pub block_ads: bool,
    /// Moves on as soon as a track you have disliked starts.
    pub skip_disliked: bool,
    /// YouTube's quality name for music videos ("auto", "hd720", "hd1080", "hd1440", "hd2160").
    pub video_quality: String,
}

/// Center frequencies (Hz) of the equalizer bands.
pub const EQ_BANDS: [u32; 10] = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
pub const EQ_MAX_GAIN: i8 = 12;
pub const VIDEO_QUALITIES: [&str; 5] = ["auto", "hd720", "hd1080", "hd1440", "hd2160"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Equalizer {
    pub enabled: bool,
    /// Gain in dB for each of `EQ_BANDS`.
    pub gains: [i8; 10],
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            continue_where_you_left_off: true,
            continue_where_you_left_off_paused: true,
            enable_speaker_fill: false,
            progress_in_taskbar: false,
            ratio_volume: false,
            show_videos: true,
            open_player_on_play: true,
            playback_rate: 100,
            crossfade_seconds: 0,
            equalizer: Equalizer::default(),
            block_ads: true,
            skip_disliked: false,
            video_quality: "auto".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Integrations {
    pub companion_server_enabled: bool,
    #[serde(rename = "companionServerCORSWildcardEnabled")]
    pub companion_server_cors_wildcard_enabled: bool,
    pub discord_presence_enabled: bool,
    /// Discord application ID used for Rich Presence. Each distribution registers its own.
    pub discord_client_id: String,
    #[serde(rename = "lastFMEnabled")]
    pub last_fm_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Shortcuts {
    pub play_pause: String,
    pub next: String,
    pub previous: String,
    pub thumbs_up: String,
    pub thumbs_down: String,
    pub volume_up: String,
    pub volume_down: String,
}

impl Shortcuts {
    /// (setting key, accelerator, remote command) for every bindable action.
    pub fn bindings(&self) -> [(&'static str, &str, &'static str); 7] {
        [
            ("playPause", &self.play_pause, "playPause"),
            ("next", &self.next, "next"),
            ("previous", &self.previous, "previous"),
            ("thumbsUp", &self.thumbs_up, "toggleLike"),
            ("thumbsDown", &self.thumbs_down, "toggleDislike"),
            ("volumeUp", &self.volume_up, "volumeUp"),
            ("volumeDown", &self.volume_down, "volumeDown"),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LastFm {
    /// Last.fm API account credentials (https://www.last.fm/api/account/create).
    pub api_key: String,
    pub secret: String,
    pub scrobble_percent: u32,
}

impl Default for LastFm {
    fn default() -> Self {
        Self { api_key: String::new(), secret: String::new(), scrobble_percent: 50 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Developer {
    pub enable_dev_tools: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub general: General,
    pub appearance: Appearance,
    pub playback: Playback,
    pub integrations: Integrations,
    pub shortcuts: Shortcuts,
    pub lastfm: LastFm,
    pub developer: Developer,
}

impl Settings {
    fn validate(&self) -> Result<(), String> {
        if !(30..=300).contains(&self.appearance.zoom) {
            return Err("zoom must be between 30 and 300".into());
        }
        if !(1..=95).contains(&self.lastfm.scrobble_percent) {
            return Err("scrobble percent must be between 1 and 95".into());
        }
        if !(50..=200).contains(&self.playback.playback_rate) {
            return Err("playback speed must be between 50% and 200%".into());
        }
        if !VIDEO_QUALITIES.contains(&self.playback.video_quality.as_str()) {
            return Err("unknown video quality".into());
        }
        if self.playback.crossfade_seconds > 12 {
            return Err("fade length must be at most 12 seconds".into());
        }
        if self.playback.equalizer.gains.iter().any(|g| g.abs() > EQ_MAX_GAIN) {
            return Err(format!("equalizer gains must be within ±{EQ_MAX_GAIN} dB"));
        }
        Ok(())
    }
}

/// Previous and new settings after a change.
pub type Change = (Arc<Settings>, Arc<Settings>);

/// JSON-file backed store; writes are atomic (temp file + rename).
pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<Arc<Settings>>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| match serde_json::from_str::<Settings>(&text) {
                Ok(s) => Some(s),
                Err(e) => {
                    log::error!("Settings file is invalid, using defaults: {e}");
                    None
                }
            })
            .unwrap_or_default();
        Self { path, current: RwLock::new(Arc::new(settings)) }
    }

    pub fn get(&self) -> Arc<Settings> {
        self.current.read().unwrap().clone()
    }

    /// Sets one value by dotted key path. Returns (old, new) when something changed.
    pub fn set(&self, key: &str, value: Value) -> Result<Option<Change>, String> {
        let mut guard = self.current.write().unwrap();
        let mut tree = serde_json::to_value(&**guard).map_err(|e| e.to_string())?;
        let pointer = format!("/{}", key.replace('.', "/"));
        let slot = tree.pointer_mut(&pointer).ok_or_else(|| format!("unknown setting '{key}'"))?;
        *slot = value;
        let next: Settings = serde_json::from_value(tree).map_err(|e| format!("invalid value for '{key}': {e}"))?;
        next.validate()?;
        if next == **guard {
            return Ok(None);
        }
        let old = guard.clone();
        let next = Arc::new(next);
        *guard = next.clone();
        drop(guard);
        self.persist(&next);
        Ok(Some((old, next)))
    }

    /// Resets one top-level section to defaults.
    pub fn reset(&self, section: &str) -> Result<Option<Change>, String> {
        let defaults = serde_json::to_value(Settings::default()).map_err(|e| e.to_string())?;
        let value = defaults.get(section).cloned().ok_or_else(|| format!("unknown section '{section}'"))?;
        self.set(section, value)
    }

    fn persist(&self, settings: &Settings) {
        if let Err(e) = write_json_atomic(&self.path, settings) {
            log::error!("Failed to save settings: {e}");
        }
    }
}

pub fn write_json_atomic<T: Serialize>(path: &std::path::Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SettingsStore {
        let dir = std::env::temp_dir().join(format!("ytmr-test-{}", uuid::Uuid::new_v4()));
        SettingsStore::load(dir.join("config.json"))
    }

    #[test]
    fn set_by_key_path_reports_change_and_persists() {
        let s = store();
        let (old, new) = s.set("general.startOnBoot", Value::Bool(true)).unwrap().unwrap();
        assert!(!old.general.start_on_boot && new.general.start_on_boot);
        assert!(s.set("general.startOnBoot", Value::Bool(true)).unwrap().is_none());
        let reloaded = SettingsStore::load(s.path.clone());
        assert!(reloaded.get().general.start_on_boot);
    }

    #[test]
    fn rejects_unknown_keys_and_bad_values() {
        let s = store();
        assert!(s.set("general.nope", Value::Bool(true)).is_err());
        assert!(s.set("appearance.zoom", Value::from(5)).is_err());
        assert!(s.set("general.startOnBoot", Value::from("yes")).is_err());
        assert!(s.set("playback.equalizer.gains.3", Value::from(13)).is_err());
        assert!(s.set("playback.equalizer.gains.3", Value::from(-6)).unwrap().is_some());
        assert_eq!(s.get().playback.equalizer.gains[3], -6);
        assert_eq!(s.get().appearance.zoom, 100);
    }

    #[test]
    fn upstream_key_names_are_preserved() {
        let v = serde_json::to_value(Settings::default()).unwrap();
        assert!(v.pointer("/integrations/companionServerCORSWildcardEnabled").is_some());
        assert!(v.pointer("/integrations/lastFMEnabled").is_some());
        assert!(v.pointer("/appearance/customCssPath").is_some());
    }
}
