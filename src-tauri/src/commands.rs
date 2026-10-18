//! Commands for the local UI windows (title bar shell, settings, mini player, auth).

use crate::integrations::{self, companion, lastfm};
use crate::lyrics::Lyrics;
use crate::memory::Memory;
use crate::player::PlayerState;
use crate::settings::Settings;
use crate::{app_ctx, windows, ytm};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_dialog::DialogExt;

type Result<T> = std::result::Result<T, String>;

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    (*app_ctx(&app).settings.get()).clone()
}

fn after_change(app: &AppHandle, change: Option<crate::settings::Change>) {
    if let Some((old, new)) = change {
        integrations::apply(app, Some(&old), &new);
    }
}

#[tauri::command]
pub fn set_setting(app: AppHandle, key: String, value: Value) -> Result<()> {
    let change = app_ctx(&app).settings.set(&key, value)?;
    after_change(&app, change);
    Ok(())
}

#[tauri::command]
pub fn reset_settings(app: AppHandle, section: String) -> Result<()> {
    let change = app_ctx(&app).settings.reset(&section)?;
    after_change(&app, change);
    Ok(())
}

#[tauri::command]
pub fn get_memory(app: AppHandle) -> Memory {
    app_ctx(&app).memory.get()
}

#[tauri::command]
pub fn get_player_state(app: AppHandle) -> PlayerState {
    (*app_ctx(&app).player.get()).clone()
}

/// Same command vocabulary and validation as the companion API `/command`, plus UI-only
/// actions that open YTM's own player page and add-to-playlist dialog, and close the track.
#[tauri::command]
pub fn player_command(app: AppHandle, command: String, data: Option<Value>) -> Result<()> {
    if command == "closeTrack" {
        ytm::close_track(&app);
        return Ok(());
    }
    if matches!(command.as_str(), "togglePlayerPage" | "addToPlaylist") {
        ytm::call(&app, &command, &[]);
        return Ok(());
    }
    let body = json!({ "command": command, "data": data });
    let cmd = companion::parse_command(&body, &app_ctx(&app).player.get()).map_err(|e| e.message().to_string())?;
    ytm::execute(&app, &cmd);
    Ok(())
}

/// Pauses playback after `minutes`; 0 clears the timer.
#[tauri::command]
pub fn sleep_timer_set(app: AppHandle, minutes: u32) -> Result<()> {
    if minutes > 24 * 60 {
        return Err("sleep timer is limited to 24 hours".into());
    }
    let ctx = app_ctx(&app);
    let ends_at = (minutes > 0).then(|| {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        (now + std::time::Duration::from_secs(u64::from(minutes) * 60)).as_millis() as u64
    });
    ctx.memory.update(&app, |m| m.sleep_timer_ends_at = ends_at);
    let Some(ends_at) = ends_at else { return Ok(()) };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(u64::from(minutes) * 60)).await;
        let ctx = app_ctx(&app);
        // A newer timer (or a clear) replaced this one.
        if ctx.memory.read(|m| m.sleep_timer_ends_at) != Some(ends_at) {
            return;
        }
        ctx.memory.update(&app, |m| m.sleep_timer_ends_at = None);
        ytm::execute(&app, &ytm::Command::Pause);
    });
    Ok(())
}

#[tauri::command]
// Window-creating commands must be async: building a window from a synchronous command
// deadlocks on Windows.
pub async fn open_settings(app: AppHandle, tab: Option<String>) -> Result<()> {
    windows::open_settings(&app, tab.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_mini_player(app: AppHandle) -> Result<()> {
    windows::toggle_mini_player(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn show_plugins_menu(app: AppHandle) -> Result<()> {
    crate::plugins_menu::show(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_fullscreen(app: AppHandle) {
    windows::toggle_fullscreen(&app);
}

#[tauri::command]
pub fn set_tour_open(app: AppHandle, open: bool) {
    windows::set_tour_open(&app, open);
}

#[tauri::command]
pub fn set_now_playing_open(app: AppHandle, open: bool) {
    windows::set_now_playing_open(&app, open);
}

/// Lyrics for the current track, synced when available.
#[tauri::command]
pub async fn get_lyrics(app: AppHandle) -> Result<Option<Lyrics>> {
    let ctx = app_ctx(&app);
    let Some(video) = ctx.player.get().video.clone().filter(|v| !v.id.is_empty() && !v.is_live) else {
        return Ok(None);
    };
    ctx.lyrics.get(&video).await
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    windows::show_main(&app);
}

#[tauri::command]
pub fn ytm_recreate(app: AppHandle) -> Result<()> {
    ytm::recreate(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ytm_navigate_home(app: AppHandle) {
    ytm::navigate_home(&app);
}

#[tauri::command]
pub fn ytm_history(app: AppHandle, forward: bool) {
    if let Some(view) = ytm::webview(&app) {
        let _ = view.eval(if forward { "history.forward()" } else { "history.back()" });
    }
}

#[tauri::command]
pub fn open_ytm_devtools(app: AppHandle) {
    if app_ctx(&app).settings.get().developer.enable_dev_tools {
        if let Some(view) = ytm::webview(&app) {
            view.open_devtools();
        }
    }
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    crate::save_state(&app);
    app.restart();
}

#[tauri::command]
pub fn companion_list(app: AppHandle) -> Vec<companion::AuthorizedCompanion> {
    companion::authorized_companions(&app)
}

#[tauri::command]
pub fn companion_revoke(app: AppHandle, id: String) {
    companion::revoke_token(&app, &id);
}

#[tauri::command]
pub fn companion_set_auth_enabled(app: AppHandle, enabled: bool) -> Result<()> {
    if enabled && !app_ctx(&app).settings.get().integrations.companion_server_enabled {
        return Err("Companion server is disabled".into());
    }
    companion::set_auth_window_enabled(&app, enabled);
    Ok(())
}

fn auth_request_id(webview: &Webview) -> Result<String> {
    webview
        .label()
        .strip_prefix(windows::AUTH_PREFIX)
        .map(str::to_string)
        .ok_or_else(|| "forbidden".to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthInfo {
    app_name: String,
    code: String,
}

#[tauri::command]
pub fn companion_auth_info(app: AppHandle, webview: Webview) -> Result<AuthInfo> {
    let id = auth_request_id(&webview)?;
    let (app_name, code) = app_ctx(&app).companion.auth_request_info(&id).ok_or("request expired")?;
    Ok(AuthInfo { app_name, code })
}

#[tauri::command]
pub fn companion_auth_result(app: AppHandle, webview: Webview, allowed: bool) -> Result<()> {
    let id = auth_request_id(&webview)?;
    app_ctx(&app).companion.resolve_auth_request(&id, allowed);
    Ok(())
}

#[tauri::command]
pub fn lastfm_signed_in() -> bool {
    lastfm::is_signed_in()
}

#[tauri::command]
pub fn lastfm_sign_out(app: AppHandle) {
    lastfm::sign_out(&app);
}

#[tauri::command]
pub async fn pick_css_file(app: AppHandle) -> Option<String> {
    let parent = app.get_webview_window(windows::SETTINGS);
    let mut dialog = app.dialog().file().add_filter("Stylesheet", &["css"]);
    if let Some(parent) = &parent {
        dialog = dialog.set_parent(parent);
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    dialog.pick_file(move |path| {
        let _ = tx.send(path.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()));
    });
    rx.await.ok().flatten()
}

#[derive(Serialize)]
pub struct AppInfo {
    version: &'static str,
    tauri: &'static str,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo { version: env!("CARGO_PKG_VERSION"), tauri: tauri::VERSION }
}
