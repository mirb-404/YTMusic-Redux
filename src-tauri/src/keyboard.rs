//! In-app keyboard controls. One key map, handled the same way whichever webview (the app
//! shell or the YTM view) has focus: each forwards matching keys here. Global shortcuts,
//! which also work in the background, live in `shortcuts.rs`.

use crate::player::RepeatMode;
use crate::{app_ctx, commands, windows, ytm};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Manager, Webview};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Binding {
    /// Key combinations as the webviews normalize them: modifiers in the order Ctrl, Alt,
    /// Shift, then the key; letters upper case; Shift only for letters and named keys.
    pub keys: &'static [&'static str],
    pub action: &'static str,
    pub description: &'static str,
}

const fn bind(keys: &'static [&'static str], action: &'static str, description: &'static str) -> Binding {
    Binding { keys, action, description }
}

pub const BINDINGS: &[Binding] = &[
    bind(&["Space", "K"], "playPause", "Play / pause"),
    bind(&["Ctrl+ArrowRight", "Shift+N"], "next", "Next track"),
    bind(&["Ctrl+ArrowLeft", "Shift+P"], "previous", "Previous track"),
    bind(&["Shift+ArrowRight"], "seekForward", "Forward 10 seconds"),
    bind(&["Shift+ArrowLeft"], "seekBack", "Back 10 seconds"),
    bind(&["Ctrl+ArrowUp", "="], "volumeUp", "Volume up"),
    bind(&["Ctrl+ArrowDown", "-"], "volumeDown", "Volume down"),
    bind(&["M"], "mute", "Mute / unmute"),
    bind(&["L"], "toggleLike", "Like"),
    bind(&["Shift+L"], "toggleDislike", "Dislike"),
    bind(&["S"], "shuffle", "Shuffle"),
    bind(&["R"], "repeat", "Repeat off / all / one"),
    bind(&["Ctrl+Shift+X"], "closeTrack", "Close the track (stop and clear the queue)"),
    bind(&["Y"], "lyrics", "Lyrics"),
    bind(&["Q"], "queue", "Player page and queue"),
    bind(&["V"], "video", "Music videos / audio only"),
    bind(&["F", "F11"], "fullscreen", "Full screen"),
    bind(&["I"], "mini", "Mini player"),
    bind(&["/", "Ctrl+K"], "search", "Search"),
    bind(&["Ctrl+1"], "home", "Home"),
    bind(&["Ctrl+2"], "explore", "Explore"),
    bind(&["Ctrl+3"], "library", "Library"),
    bind(&["Alt+ArrowLeft"], "back", "Back"),
    bind(&["Alt+ArrowRight"], "forward", "Forward"),
    bind(&["F6"], "switchFocus", "Move focus between the page and the player bar"),
    bind(&["Ctrl+,"], "settings", "Settings"),
    bind(&["?"], "help", "Keyboard controls"),
];

const SEEK_SECONDS: f64 = 10.0;

#[tauri::command]
pub fn keyboard_bindings() -> &'static [Binding] {
    BINDINGS
}

/// Runs a bound action. Only actions from the key map are accepted.
#[tauri::command]
pub async fn keyboard_action(app: AppHandle, webview: Webview, action: String) -> Result<(), String> {
    if !BINDINGS.iter().any(|b| b.action == action) {
        return Err("unknown keyboard action".into());
    }
    let ctx = app_ctx(&app);
    let state = ctx.player.get();
    let browse = |id: &str| ytm::call(&app, "exec", &[json!("navigate"), json!({ "browseEndpoint": { "browseId": id } })]);
    // With native browsing the app's own pages handle navigation and search.
    if windows::native_browse(&app) && matches!(action.as_str(), "search" | "home" | "explore" | "library" | "back" | "forward" | "queue") {
        use tauri::Emitter;
        windows::set_now_playing_open(&app, false);
        if ctx.memory.read(|m| m.video_view) == crate::memory::VideoView::Open {
            windows::leave_video_view(&app);
            if action == "back" {
                return Ok(());
            }
        }
        focus(&app, windows::SHELL);
        let _ = app.emit_to(tauri::EventTarget::webview(windows::SHELL), "browse-action", &action);
        return Ok(());
    }
    match action.as_str() {
        "mute" => ytm::execute(&app, if state.player.muted { &ytm::Command::Unmute } else { &ytm::Command::Mute }),
        "seekForward" | "seekBack" => {
            let Some(video) = &state.video else { return Ok(()) };
            let step = if action == "seekForward" { SEEK_SECONDS } else { -SEEK_SECONDS };
            let position = (state.player.video_progress + step).clamp(0.0, video.duration_seconds as f64);
            ytm::execute(&app, &ytm::Command::SeekTo(position));
        }
        "repeat" => {
            let current = state.player.queue.as_ref().map_or(RepeatMode::None, |q| q.repeat_mode);
            let next = match current {
                RepeatMode::None => "ALL",
                RepeatMode::All => "ONE",
                _ => "NONE",
            };
            ytm::execute(&app, &ytm::Command::RepeatMode(next));
        }
        "closeTrack" => ytm::close_track(&app),
        "lyrics" => windows::set_now_playing_open(&app, !ctx.memory.read(|m| m.now_playing_open)),
        "queue" => {
            windows::set_now_playing_open(&app, false);
            ytm::call(&app, "togglePlayerPage", &[]);
        }
        "video" => commands::set_setting(app.clone(), "playback.showVideos".into(), json!(!ctx.settings.get().playback.show_videos))?,
        "fullscreen" => windows::toggle_fullscreen(&app),
        "mini" => windows::toggle_mini_player(&app).map_err(|e| e.to_string())?,
        "search" => {
            windows::set_now_playing_open(&app, false);
            focus(&app, ytm::LABEL);
            ytm::call(&app, "focusSearch", &[]);
        }
        "home" => ytm::navigate_home(&app),
        "explore" => browse("FEmusic_explore"),
        "library" => browse("FEmusic_library_landing"),
        "back" | "forward" => commands::ytm_history(app.clone(), action == "forward"),
        "switchFocus" => focus(&app, if webview.label() == ytm::LABEL { windows::SHELL } else { ytm::LABEL }),
        "settings" => windows::open_settings(&app, None).map_err(|e| e.to_string())?,
        "help" => windows::open_settings(&app, Some("shortcuts")).map_err(|e| e.to_string())?,
        // The rest are player commands of the same name.
        name => {
            if let Some(cmd) = ytm::Command::simple(name) {
                ytm::execute(&app, &cmd);
            }
        }
    }
    Ok(())
}

fn focus(app: &AppHandle, label: &str) {
    if let Some(view) = app.get_webview(label) {
        let _ = view.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_bound_once() {
        let mut seen = std::collections::HashSet::new();
        for key in BINDINGS.iter().flat_map(|b| b.keys) {
            assert!(seen.insert(key), "{key} is bound twice");
        }
    }

    #[test]
    fn plain_actions_are_player_commands() {
        let app_actions = [
            "mute", "seekForward", "seekBack", "repeat", "closeTrack", "lyrics", "queue", "video", "fullscreen", "mini", "search", "home", "explore",
            "library", "back", "forward", "switchFocus", "settings", "help",
        ];
        for b in BINDINGS.iter().filter(|b| !app_actions.contains(&b.action)) {
            assert!(ytm::Command::simple(b.action).is_some(), "{} does nothing", b.action);
        }
    }
}
