//! Integrations and the single place where settings changes become side effects.

pub mod companion;
pub mod custom_css;
pub mod discord;
pub mod lastfm;
pub mod notifications;

use crate::settings::Settings;
use crate::{app_ctx, shortcuts, taskbar, tray, ytm};
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tauri_plugin_autostart::ManagerExt;

/// Background tasks of task-based integrations, keyed by name.
#[derive(Default)]
pub struct Tasks(Mutex<HashMap<&'static str, JoinHandle<()>>>);

impl Tasks {
    fn set<F: Future<Output = ()> + Send + 'static>(&self, name: &'static str, enabled: bool, start: impl FnOnce() -> F) {
        let mut tasks = self.0.lock().unwrap();
        if let Some(task) = tasks.remove(name) {
            task.abort();
        }
        if enabled {
            tasks.insert(name, tauri::async_runtime::spawn(start()));
            log::info!("Integration enabled: {name}");
        }
    }
}

fn restart_required(old: &Settings, new: &Settings) -> bool {
    old.general.disable_hardware_acceleration != new.general.disable_hardware_acceleration
        || old.playback.enable_speaker_fill != new.playback.enable_speaker_fill
        || old.developer.enable_dev_tools != new.developer.enable_dev_tools
}

/// Applies settings side effects. `old` is `None` at startup, so everything enabled is applied.
pub fn apply(app: &AppHandle, old: Option<&Settings>, new: &Settings) {
    let ctx = app_ctx(app);
    let changed = |f: fn(&Settings) -> bool| old.is_none_or(|o| f(o) != f(new));

    if let Some(old) = old {
        let _ = app.emit("settings-changed", new);
        if restart_required(old, new) {
            ctx.memory.update(app, |m| m.restart_required = true);
        }
    }

    if changed(|s| s.general.start_on_boot) && !cfg!(debug_assertions) {
        let autolaunch = app.autolaunch();
        let result = if new.general.start_on_boot { autolaunch.enable() } else { autolaunch.disable() };
        if let Err(e) = result {
            log::warn!("Autostart update failed: {e}");
        }
    }

    if changed(|s| s.general.show_notification_on_song_change) {
        let app2 = app.clone();
        ctx.tasks.set("notifications", new.general.show_notification_on_song_change, || notifications::run(app2));
    }

    if let Some(previous) = old {
        if previous.appearance.zoom != new.appearance.zoom {
            if let Some(view) = ytm::webview(app) {
                let _ = view.set_zoom(f64::from(new.appearance.zoom) / 100.0);
            }
        }
        if changed(|s| s.appearance.always_show_volume_slider) {
            ytm::call(app, "setVolumeSliderPersist", &[json!(new.appearance.always_show_volume_slider)]);
        }
        if changed(|s| s.appearance.modern_ui) {
            let css = ytm::modern_css(app).map_or(serde_json::Value::Null, |c| json!(c));
            ytm::call(app, "setModernUi", &[css]);
            crate::windows::layout_main(app);
        }
        if changed(|s| s.appearance.up_next) {
            ytm::call(app, "setUpNext", &[json!(new.appearance.up_next)]);
        }
        if changed(|s| s.appearance.native_browse || s.appearance.modern_ui) {
            ytm::call(app, "setNative", &[json!(crate::windows::native_browse(app))]);
            crate::windows::layout_main(app);
        }
        if changed(|s| s.playback.block_ads) {
            ytm::call(app, "setAdBlock", &[json!(new.playback.block_ads)]);
        }
        if changed(|s| s.playback.ratio_volume) {
            let f = if new.playback.ratio_volume { "ratioVolume.enable" } else { "ratioVolume.disable" };
            ytm::call(app, f, &[]);
        }
        let audio = ytm::audio_config(new);
        if ytm::audio_config(previous) != audio {
            ytm::call(app, "setAudio", &[audio]);
        }
        if changed(|s| s.playback.progress_in_taskbar) && !new.playback.progress_in_taskbar {
            taskbar::clear_progress(app);
        }
    }

    if old.is_none_or(|o| o.appearance.custom_css_enabled != new.appearance.custom_css_enabled || o.appearance.custom_css_path != new.appearance.custom_css_path) {
        custom_css::refresh(app);
    }

    if old.is_some_and(|o| o.appearance.tray_icon_style != new.appearance.tray_icon_style) {
        tray::refresh_icon(app);
    }

    // Companion server: toggled, or restarted when the CORS policy changes while running.
    let companion_toggled = changed(|s| s.integrations.companion_server_enabled);
    let cors_changed = old.is_some_and(|o| o.integrations.companion_server_cors_wildcard_enabled != new.integrations.companion_server_cors_wildcard_enabled);
    if companion_toggled || (cors_changed && new.integrations.companion_server_enabled) {
        let app2 = app.clone();
        let enabled = new.integrations.companion_server_enabled;
        tauri::async_runtime::spawn(async move {
            companion::disable(&app2).await;
            if enabled {
                companion::enable(&app2).await;
            } else {
                companion::set_auth_window_enabled(&app2, false);
            }
        });
    }

    let discord_cfg = old.is_none_or(|o| {
        o.integrations.discord_presence_enabled != new.integrations.discord_presence_enabled
            || o.integrations.discord_client_id != new.integrations.discord_client_id
    });
    if discord_cfg {
        let app2 = app.clone();
        ctx.memory.update(app, |m| m.discord_presence_connection_failed = false);
        ctx.tasks.set("discord", new.integrations.discord_presence_enabled, || discord::run(app2));
    }

    let lastfm_cfg = old.is_none_or(|o| o.integrations.last_fm_enabled != new.integrations.last_fm_enabled || o.lastfm.api_key != new.lastfm.api_key || o.lastfm.secret != new.lastfm.secret);
    if lastfm_cfg {
        let app2 = app.clone();
        ctx.tasks.set("lastfm", new.integrations.last_fm_enabled, || lastfm::run(app2));
        if !new.integrations.last_fm_enabled {
            ctx.memory.update(app, |m| m.lastfm_status = None);
        }
    }

    if old.is_none_or(|o| o.shortcuts != new.shortcuts) {
        shortcuts::register(app, &new.shortcuts);
    }
}
