const BRIDGE_COMMANDS: &[&str] = &[
    "ytm_get_config",
    "ytm_loaded",
    "ytm_video_progress",
    "ytm_video_state",
    "ytm_video_data",
    "ytm_store_state",
    "ytm_playlist_created",
    "ytm_playlist_deleted",
    "ytm_playlists_response",
    "ytm_url_changed",
    "ytm_fullscreen",
    "ytm_toggle_fullscreen",
    "ytm_window_action",
];

const UI_COMMANDS: &[&str] = &[
    "get_settings",
    "set_setting",
    "reset_settings",
    "get_memory",
    "get_player_state",
    "player_command",
    "sleep_timer_set",
    "open_settings",
    "toggle_mini_player",
    "show_plugins_menu",
    "toggle_fullscreen",
    "set_now_playing_open",
    "get_lyrics",
    "ytm_set_caption",
    "show_main_window",
    "ytm_recreate",
    "ytm_navigate_home",
    "ytm_history",
    "open_ytm_devtools",
    "restart_app",
    "quit_app",
    "companion_list",
    "companion_revoke",
    "companion_set_auth_enabled",
    "lastfm_signed_in",
    "lastfm_sign_out",
    "pick_css_file",
    "app_info",
];

const AUTH_COMMANDS: &[&str] = &["companion_auth_info", "companion_auth_result"];

fn main() {
    let commands: Vec<&str> = [BRIDGE_COMMANDS, UI_COMMANDS, AUTH_COMMANDS].concat();
    let commands: &'static [&'static str] = Box::leak(commands.into_boxed_slice());
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(commands)))
        .expect("failed to run tauri-build");
}
