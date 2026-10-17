//! Window management: main window (title bar shell + YTM view), settings, mini player
//! and companion authorization windows.

use crate::{app_ctx, state_store::WindowBounds, ytm};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewBuilder, WebviewUrl,
    WebviewWindowBuilder, Window, WindowEvent,
};

pub const MAIN: &str = "main";
pub const SHELL: &str = "shell";
pub const SETTINGS: &str = "settings";
pub const MINI: &str = "mini";
pub const AUTH_PREFIX: &str = "companion-auth-";
/// Height of the custom title bar rendered by the shell webview, in logical pixels.
pub const TITLE_BAR_HEIGHT: f64 = 40.0;
/// Height of the app-rendered player bar (Modern UI), in logical pixels.
pub const PLAYER_BAR_HEIGHT: f64 = 88.0;

pub fn create_main(app: &AppHandle) -> tauri::Result<()> {
    let ctx = app_ctx(app);
    let (bounds, maximized) = ctx.state.read(|s| (s.window_bounds, s.window_maximized));
    let window = tauri::window::WindowBuilder::new(app, MAIN)
        .title("YTMusic Redux")
        .decorations(false)
        .inner_size(1280.0, 760.0)
        .min_inner_size(320.0, 240.0)
        .visible(false)
        .build()?;
    if let Some(b) = bounds {
        let _ = window.set_position(PhysicalPosition::new(b.x, b.y));
        let _ = window.set_size(PhysicalSize::new(b.width, b.height));
    } else {
        let _ = window.center();
    }
    if maximized {
        let _ = window.maximize();
    }

    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    let shell = WebviewBuilder::new(SHELL, WebviewUrl::App("index.html?view=shell".into()))
        .additional_browser_args(&ctx.browser_args)
        .auto_resize();
    window.add_child(shell, LogicalPosition::new(0.0, 0.0), size)?;

    let app_handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Resized(_) => {
            layout_main(&app_handle);
            emit_window_state(&app_handle);
        }
        WindowEvent::CloseRequested { api, .. } => {
            save_main_bounds(&app_handle);
            if app_ctx(&app_handle).settings.get().general.hide_to_tray_on_close {
                api.prevent_close();
                if let Some(w) = app_handle.get_window(MAIN) {
                    let _ = w.hide();
                }
            } else {
                crate::quit(&app_handle);
            }
        }
        _ => {}
    });

    if !ctx.settings.get().general.start_minimized {
        window.show()?;
    }
    Ok(())
}

pub fn emit_window_state(app: &AppHandle) {
    use tauri::Emitter;
    if let Some(w) = app.get_window(MAIN) {
        let maximized = w.is_maximized().unwrap_or(false);
        let _ = app.emit_to(
            tauri::EventTarget::webview(SHELL),
            "window-state",
            serde_json::json!({ "maximized": maximized, "fullscreen": w.is_fullscreen().unwrap_or(false) }),
        );
        ytm::call(app, "setMaximized", &[serde_json::json!(maximized)]);
    }
}

/// Fullscreen requested by YTM's video player, which covers the whole window. App
/// fullscreen (F11) only hides the title bar and keeps the player bar.
static VIDEO_FULLSCREEN: AtomicBool = AtomicBool::new(false);

/// Positions the YTM view between the title bar and the player bar, and shows it unless
/// it is still loading or the now playing view covers it.
pub fn layout_main(app: &AppHandle) {
    let (Some(window), Some(view)) = (app.get_window(MAIN), ytm::webview(app)) else { return };
    let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) else { return };
    let size = size.to_logical::<f64>(scale);
    let ctx = app_ctx(app);
    let fullscreen = window.is_fullscreen().unwrap_or(false);
    let video = fullscreen && VIDEO_FULLSCREEN.load(Ordering::Relaxed);
    let owns_header = ctx.memory.read(|m| m.ytm_owns_header);
    let top = if fullscreen || owns_header { 0.0 } else { TITLE_BAR_HEIGHT };
    let bottom = if !video && ctx.settings.get().appearance.modern_ui { PLAYER_BAR_HEIGHT } else { 0.0 };
    let _ = view.set_position(LogicalPosition::new(0.0, top));
    let _ = view.set_size(LogicalSize::new(size.width, (size.height - top - bottom).max(1.0)));
    let (loading, covered) = ctx.memory.read(|m| (m.ytm_view_loading, m.now_playing_open && !video));
    let _ = if loading || covered { view.hide() } else { view.show() };
}

fn apply_fullscreen(app: &AppHandle, fullscreen: bool, video: bool) {
    if let Some(w) = app.get_window(MAIN) {
        VIDEO_FULLSCREEN.store(fullscreen && video, Ordering::Relaxed);
        let _ = w.set_fullscreen(fullscreen);
        layout_main(app);
        emit_window_state(app);
    }
}

/// Video fullscreen, entered and left by YTM's player.
pub fn set_video_fullscreen(app: &AppHandle, fullscreen: bool) {
    apply_fullscreen(app, fullscreen, true);
}

/// App fullscreen (F11).
pub fn toggle_fullscreen(app: &AppHandle) {
    let on = app.get_window(MAIN).is_some_and(|w| !w.is_fullscreen().unwrap_or(false));
    apply_fullscreen(app, on, false);
}

/// Shows or hides the app's now playing view in place of the YTM view.
pub fn set_now_playing_open(app: &AppHandle, open: bool) {
    app_ctx(app).memory.update(app, |m| m.now_playing_open = open);
    layout_main(app);
}

fn save_main_bounds(app: &AppHandle) {
    let Some(w) = app.get_window(MAIN) else { return };
    let maximized = w.is_maximized().unwrap_or(false);
    let bounds = (!maximized && !w.is_minimized().unwrap_or(false))
        .then(|| Some((w.outer_position().ok()?, w.inner_size().ok()?)))
        .flatten()
        .map(|(p, s)| WindowBounds { x: p.x, y: p.y, width: s.width, height: s.height });
    app_ctx(app).state.update_volatile(|s| {
        if bounds.is_some() {
            s.window_bounds = bounds;
        }
        s.window_maximized = maximized;
    });
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_window(MAIN) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn toggle_main(app: &AppHandle) {
    if let Some(w) = app.get_window(MAIN) {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_main(app);
        }
    }
}

fn centered_on_main(app: &AppHandle, width: f64, height: f64) -> Option<(f64, f64)> {
    let main: Window = app.get_window(MAIN)?;
    let scale = main.scale_factor().ok()?;
    let pos = main.outer_position().ok()?.to_logical::<f64>(scale);
    let size = main.outer_size().ok()?.to_logical::<f64>(scale);
    Some((pos.x + (size.width - width) / 2.0, pos.y + (size.height - height) / 2.0))
}

fn ui_window<'a>(app: &'a AppHandle, label: &'a str, view: &str) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    let ctx = app_ctx(app);
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(format!("index.html?view={view}").into()))
        .decorations(false)
        .additional_browser_args(&ctx.browser_args)
        .devtools(ctx.settings.get().developer.enable_dev_tools || cfg!(debug_assertions))
}

/// Opens (or focuses) the settings window, optionally on a specific tab.
pub fn open_settings(app: &AppHandle, tab: Option<&str>) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(SETTINGS) {
        if let Some(tab) = tab {
            use tauri::Emitter;
            let _ = app.emit_to(tauri::EventTarget::webview(SETTINGS), "settings-tab", tab);
        }
        return w.set_focus();
    }
    let (w, h) = (900.0, 640.0);
    let view = match tab {
        Some(tab) => format!("settings&tab={tab}"),
        None => "settings".into(),
    };
    let mut builder = ui_window(app, SETTINGS, &view).title("Settings").inner_size(w, h).min_inner_size(720.0, 480.0);
    if let Some((x, y)) = centered_on_main(app, w, h) {
        builder = builder.position(x, y);
    }
    builder.build()?;
    Ok(())
}

pub fn toggle_mini_player(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(MINI) {
        return w.close();
    }
    ui_window(app, MINI, "mini")
        .title("Mini Player")
        .inner_size(380.0, 156.0)
        .resizable(false)
        .maximizable(false)
        .always_on_top(true)
        .build()?;
    Ok(())
}

pub fn open_companion_auth(app: &AppHandle, request_id: &str) -> tauri::Result<()> {
    let label = format!("{AUTH_PREFIX}{request_id}");
    let (w, h) = (520.0, 440.0);
    let mut builder = ui_window(app, &label, &format!("auth&request={request_id}"))
        .title("Companion Authorization")
        .inner_size(w, h)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .always_on_top(true)
        .focused(true);
    if let Some((x, y)) = centered_on_main(app, w, h) {
        builder = builder.position(x, y);
    }
    let window = builder.build()?;
    let _ = window.request_user_attention(Some(tauri::UserAttentionType::Informational));
    Ok(())
}

pub fn close_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.close();
    }
}
