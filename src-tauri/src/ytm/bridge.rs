//! Commands invoked by the injected bridge script (remote origin music.youtube.com).
//!
//! The remote capability only grants these commands, and each one additionally checks
//! that the caller is the YTM webview, mirroring upstream's `event.sender` checks.

use super::{BridgeConfig, LABEL};
use crate::{app_ctx, integrations::companion, windows};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Runtime, Webview};

pub const SCRIPT: &str = include_str!("bridge.js");

type Result<T> = std::result::Result<T, String>;

fn ensure_ytm<R: Runtime>(webview: &Webview<R>) -> Result<()> {
    if webview.label() == LABEL {
        Ok(())
    } else {
        Err("forbidden".into())
    }
}

#[tauri::command]
pub fn ytm_get_config(webview: Webview, app: AppHandle) -> Result<BridgeConfig> {
    ensure_ytm(&webview)?;
    let ctx = app_ctx(&app);
    let s = ctx.settings.get();
    let (last_url, last_video_id, last_playlist_id) =
        ctx.state.read(|st| (st.last_url.clone(), st.last_video_id.clone(), st.last_playlist_id.clone()));
    Ok(BridgeConfig {
        continue_where_you_left_off: s.playback.continue_where_you_left_off,
        last_url,
        last_video_id,
        last_playlist_id,
        always_show_volume_slider: s.appearance.always_show_volume_slider,
        ratio_volume: s.playback.ratio_volume,
        custom_css: crate::integrations::custom_css::current_css(&app),
        modern_css: super::modern_css(&app),
        audio: super::audio_config(&s),
    })
}

#[tauri::command]
pub fn ytm_loaded(webview: Webview, app: AppHandle, owns_header: bool) -> Result<()> {
    ensure_ytm(&webview)?;
    app_ctx(&app).memory.update(&app, |m| m.ytm_owns_header = owns_header);
    super::mark_loaded(&app);
    windows::emit_window_state(&app);
    Ok(())
}

#[tauri::command]
pub async fn ytm_video_progress(webview: Webview, app: AppHandle, progress: f64) -> Result<()> {
    ensure_ytm(&webview)?;
    if progress.is_finite() && progress >= 0.0 {
        app_ctx(&app).player.update_progress(progress);
    }
    Ok(())
}

#[tauri::command]
pub async fn ytm_video_state(webview: Webview, app: AppHandle, state: i64) -> Result<()> {
    ensure_ytm(&webview)?;
    app_ctx(&app).player.update_track_state(state);
    Ok(())
}

#[tauri::command]
pub async fn ytm_video_data(
    webview: Webview,
    app: AppHandle,
    video_details: Value,
    playlist_id: Option<String>,
    album: Option<Value>,
    like_status: Option<String>,
    has_full_metadata: bool,
) -> Result<()> {
    ensure_ytm(&webview)?;
    let ctx = app_ctx(&app);
    let video_id = video_details.get("videoId").and_then(Value::as_str).unwrap_or_default().to_string();
    ctx.state.update_volatile(|s| {
        s.last_video_id = video_id;
        s.last_playlist_id = playlist_id.clone().unwrap_or_default();
    });
    ctx.player.update_video(&video_details, playlist_id, album.as_ref(), like_status.as_deref(), has_full_metadata);
    Ok(())
}

#[tauri::command]
pub async fn ytm_store_state(
    webview: Webview,
    app: AppHandle,
    queue: Option<Value>,
    like_status: Option<String>,
    volume: Option<f64>,
    muted: bool,
    ad_playing: bool,
) -> Result<()> {
    ensure_ytm(&webview)?;
    app_ctx(&app).player.update_from_store(queue.as_ref(), like_status.as_deref(), volume, muted, ad_playing);
    Ok(())
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Playlist {
    pub id: String,
    pub title: String,
}

#[tauri::command]
pub fn ytm_playlist_created(webview: Webview, app: AppHandle, playlist: Playlist) -> Result<()> {
    ensure_ytm(&webview)?;
    companion::emit_realtime(&app, "playlist-created", &playlist);
    Ok(())
}

#[tauri::command]
pub fn ytm_playlist_deleted(webview: Webview, app: AppHandle, playlist_id: String) -> Result<()> {
    ensure_ytm(&webview)?;
    companion::emit_realtime(&app, "playlist-deleted", &playlist_id);
    Ok(())
}

#[tauri::command]
pub fn ytm_playlists_response(webview: Webview, app: AppHandle, request_id: String, playlists: Vec<Playlist>) -> Result<()> {
    ensure_ytm(&webview)?;
    companion::resolve_playlists(&app, &request_id, playlists);
    Ok(())
}

#[tauri::command]
pub async fn ytm_url_changed(webview: Webview, app: AppHandle, url: String) -> Result<()> {
    ensure_ytm(&webview)?;
    if url.starts_with(super::YTM_HOME) {
        app_ctx(&app).state.update_volatile(|s| s.last_url = url);
    }
    Ok(())
}

#[tauri::command]
pub fn ytm_fullscreen(webview: Webview, app: AppHandle, fullscreen: bool) -> Result<()> {
    ensure_ytm(&webview)?;
    windows::set_video_fullscreen(&app, fullscreen);
    Ok(())
}

/// Window controls drawn into YTM's header, which replaces the app title bar. Async because
/// opening settings or the mini player creates a window.
#[tauri::command]
pub async fn ytm_window_action(webview: Webview, app: AppHandle, action: String) -> Result<()> {
    ensure_ytm(&webview)?;
    let window = webview.window();
    let result = match action.as_str() {
        "drag" => window.start_dragging(),
        "minimize" => window.minimize(),
        "toggleMaximize" => match window.is_maximized() {
            Ok(true) => window.unmaximize(),
            _ => window.maximize(),
        },
        "close" => window.close(),
        "toggleFullscreen" => {
            windows::toggle_fullscreen(&app);
            Ok(())
        }
        "settings" => windows::open_settings(&app, None),
        "mini" => windows::toggle_mini_player(&app),
        "plugins" => crate::plugins_menu::show(&app),
        _ => return Err("unknown window action".into()),
    };
    result.map_err(|e| e.to_string())
}

/// F11 pressed while the YTM view has focus.
#[tauri::command]
pub fn ytm_toggle_fullscreen(webview: Webview, app: AppHandle) -> Result<()> {
    ensure_ytm(&webview)?;
    windows::toggle_fullscreen(&app);
    Ok(())
}
