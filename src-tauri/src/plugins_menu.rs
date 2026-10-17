//! Title-bar "Plugins" menu: native popup with a toggle per integration.
//!
//! A native menu is used because the YTM webview covers the area below the title bar,
//! which would hide an HTML dropdown.

use crate::{app_ctx, integrations, windows};
use serde_json::Value;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Manager};

const PREFIX: &str = "plugin:";
const SETTINGS_ID: &str = "plugin-settings";

/// (setting key, label) for every toggleable plugin.
const PLUGINS: [(&str, &str); 9] = [
    ("playback.showVideos", "Show music videos"),
    ("appearance.lyricsCaptions", "Lyric captions"),
    ("playback.equalizer.enabled", "Equalizer"),
    ("integrations.companionServerEnabled", "Companion server"),
    ("integrations.discordPresenceEnabled", "Discord Rich Presence"),
    ("integrations.lastFMEnabled", "Last.fm scrobbling"),
    ("general.showNotificationOnSongChange", "Song change notifications"),
    ("appearance.customCssEnabled", "Custom CSS"),
    ("playback.ratioVolume", "Ratio volume"),
];

fn enabled(settings: &Value, key: &str) -> bool {
    settings.pointer(&format!("/{}", key.replace('.', "/"))).and_then(Value::as_bool).unwrap_or(false)
}

pub fn show(app: &AppHandle) -> tauri::Result<()> {
    let settings = serde_json::to_value(&*app_ctx(app).settings.get()).unwrap_or_default();
    let menu = Menu::new(app)?;
    for (key, label) in PLUGINS {
        let item = CheckMenuItem::with_id(app, format!("{PREFIX}{key}"), label, true, enabled(&settings, key), None::<&str>)?;
        menu.append(&item)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, SETTINGS_ID, "Plugin settings…", true, None::<&str>)?)?;
    if let Some(window) = app.get_window(windows::MAIN) {
        window.popup_menu(&menu)?;
    }
    Ok(())
}

pub fn handle_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id.as_ref();
    if id == SETTINGS_ID {
        let _ = windows::open_settings(app, Some("integrations"));
        return;
    }
    let Some(key) = id.strip_prefix(PREFIX) else { return };
    if !PLUGINS.iter().any(|(k, _)| *k == key) {
        return;
    }
    let ctx = app_ctx(app);
    let current = enabled(&serde_json::to_value(&*ctx.settings.get()).unwrap_or_default(), key);
    match ctx.settings.set(key, Value::Bool(!current)) {
        Ok(Some((old, new))) => integrations::apply(app, Some(&old), &new),
        Ok(None) => {}
        Err(e) => log::warn!("Plugin toggle failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_plugin_key_is_a_boolean_setting() {
        let settings = serde_json::to_value(crate::settings::Settings::default()).unwrap();
        for (key, _) in PLUGINS {
            let pointer = format!("/{}", key.replace('.', "/"));
            assert!(settings.pointer(&pointer).is_some_and(Value::is_boolean), "{key}");
        }
    }
}
