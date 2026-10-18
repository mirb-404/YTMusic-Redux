//! YTMusic Redux: a Tauri reimplementation of YouTube Music Desktop App.

mod commands;
mod integrations;
mod lyrics;
mod memory;
mod plugins_menu;
mod player;
mod settings;
mod shortcuts;
mod state_store;
mod taskbar;
mod tray;
mod windows;
mod ytm;

use memory::MemoryStore;
use player::PlayerStore;
use settings::SettingsStore;
use state_store::StateStore;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, EventTarget, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

/// Shared application state. Each field owns one concern; see the module docs.
pub struct AppCtx {
    pub settings: SettingsStore,
    pub state: StateStore,
    pub memory: MemoryStore,
    pub player: PlayerStore,
    pub companion: integrations::companion::Companion,
    pub tasks: integrations::Tasks,
    pub css_watcher: Mutex<Option<notify::RecommendedWatcher>>,
    pub lyrics: lyrics::Cache,
    /// WebView2 arguments. All webviews share one browser environment, so they must match.
    pub browser_args: String,
}

pub fn app_ctx(app: &AppHandle) -> tauri::State<'_, AppCtx> {
    app.state::<AppCtx>()
}

fn browser_args(s: &settings::Settings) -> String {
    // Casting is unsupported, so the media router is off: without it Chromium never offers a
    // cast button, not even before the bridge has hidden YTM's own.
    let mut args = String::from("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,MediaRouter,GlobalMediaControlsCastStartStop");
    args.push_str(if s.playback.continue_where_you_left_off_paused {
        " --autoplay-policy=document-user-activation-required"
    } else {
        " --autoplay-policy=no-user-gesture-required"
    });
    if s.general.disable_hardware_acceleration {
        args.push_str(" --disable-gpu");
    }
    if s.playback.enable_speaker_fill {
        args.push_str(" --try-supported-channel-layouts");
    }
    args
}

pub fn save_state(app: &AppHandle) {
    app_ctx(app).state.save();
}

pub fn quit(app: &AppHandle) {
    save_state(app);
    app.exit(0);
}

/// Handles `ytmr://play/<videoId>[/<playlistId>]` and shared links: a YouTube Music,
/// YouTube or youtu.be URL with `ytmr://` in place of `https://`.
fn handle_deep_link(app: &AppHandle, url: &str) {
    log::info!("Handling protocol url");
    let Some(rest) = url.split("://").nth(1) else { return };
    let mut parts = rest.trim_end_matches('/').split('/');
    if parts.next() == Some("play") {
        if let Some(video_id) = parts.next().filter(|v| !v.is_empty()) {
            let playlist_id = parts.next().map(str::to_string);
            ytm::execute(app, &ytm::Command::ChangeVideo { video_id: Some(video_id.to_string()), playlist_id });
        }
        return;
    }
    let Some(link) = url.parse().ok().as_ref().and_then(ytm::shared_link) else { return };
    match link {
        ytm::SharedLink::Track { video_id, playlist_id } => {
            ytm::execute(app, &ytm::Command::ChangeVideo { video_id: Some(video_id), playlist_id });
        }
        ytm::SharedLink::Page(page) => {
            if let Some(view) = ytm::webview(app) {
                let _ = view.navigate(page);
            }
        }
    }
    if let Some(w) = app.get_window(windows::MAIN) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let config_dir = app.path().app_config_dir()?;
    let settings = SettingsStore::load(config_dir.join("config.json"));
    let browser_args = browser_args(&settings.get());
    app.manage(AppCtx {
        settings,
        state: StateStore::load(config_dir.join("state.json")),
        memory: MemoryStore::new(),
        player: PlayerStore::new(),
        companion: Default::default(),
        tasks: Default::default(),
        css_watcher: Mutex::new(None),
        lyrics: Default::default(),
        browser_args,
    });
    let ctx = app_ctx(&handle);
    let tour_open = !ctx.state.read(|s| s.tour_seen);
    ctx.memory.update(&handle, |m| {
        m.secure_storage_available = state_store::secrets::available();
        m.tour_open = tour_open;
    });

    tray::create(&handle)?;
    app.on_menu_event(plugins_menu::handle_event);
    windows::create_main(&handle)?;
    ytm::create(&handle)?;
    integrations::apply(&handle, None, &ctx.settings.get());
    tauri::async_runtime::spawn(taskbar::run(handle.clone()));

    // Forward player state to the title bar and mini player.
    let forward_app = handle.clone();
    tauri::async_runtime::spawn(async move {
        let mut rx = app_ctx(&forward_app).player.subscribe();
        while rx.changed().await.is_ok() {
            let state = rx.borrow_and_update().clone();
            for label in [windows::SHELL, windows::MINI] {
                let _ = forward_app.emit_to(EventTarget::webview(label), "player-state", &*state);
            }
        }
    });

    // Persist frequently-changing state every 5 minutes.
    let saver_app = handle.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(300)).await;
            save_state(&saver_app);
        }
    });

    #[cfg(any(windows, target_os = "linux"))]
    if let Err(e) = app.deep_link().register_all() {
        log::warn!("Deep link registration failed: {e}");
    }
    let link_app = handle.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            handle_deep_link(&link_app, url.as_str());
        }
    });
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        for url in urls {
            handle_deep_link(&handle, url.as_str());
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_main(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            ytm::bridge::ytm_get_config,
            ytm::bridge::ytm_loaded,
            ytm::bridge::ytm_video_progress,
            ytm::bridge::ytm_video_state,
            ytm::bridge::ytm_video_data,
            ytm::bridge::ytm_store_state,
            ytm::bridge::ytm_playlist_created,
            ytm::bridge::ytm_playlist_deleted,
            ytm::bridge::ytm_playlists_response,
            ytm::bridge::ytm_url_changed,
            ytm::bridge::ytm_fullscreen,
            ytm::bridge::ytm_toggle_fullscreen,
            ytm::bridge::ytm_window_action,
            commands::get_settings,
            commands::set_setting,
            commands::reset_settings,
            commands::get_memory,
            commands::get_player_state,
            commands::player_command,
            commands::sleep_timer_set,
            commands::open_settings,
            commands::toggle_mini_player,
            commands::show_plugins_menu,
            commands::toggle_fullscreen,
            commands::set_now_playing_open,
            commands::set_tour_open,
            commands::get_lyrics,
            commands::show_main_window,
            commands::ytm_recreate,
            commands::ytm_navigate_home,
            commands::ytm_history,
            commands::open_ytm_devtools,
            commands::restart_app,
            commands::quit_app,
            commands::companion_list,
            commands::companion_revoke,
            commands::companion_set_auth_enabled,
            commands::companion_auth_info,
            commands::companion_auth_result,
            commands::lastfm_signed_in,
            commands::lastfm_sign_out,
            commands::pick_css_file,
            commands::app_info,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                save_state(app);
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn browser_args_follow_settings() {
        let mut s = crate::settings::Settings::default();
        assert!(super::browser_args(&s).contains("document-user-activation-required"));
        s.general.disable_hardware_acceleration = true;
        s.playback.enable_speaker_fill = true;
        s.playback.continue_where_you_left_off_paused = false;
        let args = super::browser_args(&s);
        assert!(args.contains("--disable-gpu") && args.contains("--try-supported-channel-layouts") && args.contains("no-user-gesture-required"));
    }
}
