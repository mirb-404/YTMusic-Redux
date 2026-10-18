//! Window management: main window (title bar shell + YTM view), settings, mini player
//! and companion authorization windows.

use crate::{app_ctx, memory::VideoView, state_store::WindowBounds, ytm};
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
/// Height of the native pages' top bar (back, search, window controls), which stays above
/// the video view.
pub const NATIVE_TOP_BAR_HEIGHT: f64 = 64.0;
/// Height of the app-rendered player bar (Modern UI), in logical pixels.
pub const PLAYER_BAR_HEIGHT: f64 = 88.0;
/// The app's background (`--bg`). Window and webviews paint it before content arrives, so
/// resizing, maximizing and restoring never flash white.
pub const BACKGROUND: tauri::window::Color = tauri::window::Color(11, 11, 15, 255);

pub fn create_main(app: &AppHandle) -> tauri::Result<()> {
    let ctx = app_ctx(app);
    let (bounds, maximized) = ctx.state.read(|s| (s.window_bounds, s.window_maximized));
    let window = tauri::window::WindowBuilder::new(app, MAIN)
        .title("YTMusic Redux")
        .decorations(false)
        .inner_size(1280.0, 760.0)
        .min_inner_size(320.0, 240.0)
        .background_color(BACKGROUND)
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
        .background_color(BACKGROUND)
        .auto_resize();
    window.add_child(shell, LogicalPosition::new(0.0, 0.0), size)?;

    let app_handle = app.clone();
    window.on_window_event(move |event| match event {
        // Minimizing resizes the window to nothing. Laying out for that would squash YTM into a
        // 1px view that reflows and pops back in on restore, so the layout is kept as it was.
        WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
            set_main_in_background(&app_handle, false);
            layout_main(&app_handle);
            emit_window_state(&app_handle);
        }
        WindowEvent::Resized(_) => set_main_in_background(&app_handle, true),
        WindowEvent::Focused(true) => set_main_in_background(&app_handle, false),
        WindowEvent::CloseRequested { api, .. } => {
            save_main_bounds(&app_handle);
            if app_ctx(&app_handle).settings.get().general.hide_to_tray_on_close {
                api.prevent_close();
                if let Some(w) = app_handle.get_window(MAIN) {
                    let _ = w.hide();
                }
                set_main_in_background(&app_handle, true);
            } else {
                crate::quit(&app_handle);
            }
        }
        _ => {}
    });

    if ctx.settings.get().general.start_minimized {
        set_main_in_background(app, true);
    } else {
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
            serde_json::json!({ "maximized": maximized, "fullscreen": is_fullscreen() }),
        );
        ytm::call(app, "setMaximized", &[serde_json::json!(maximized)]);
        ytm::call(app, "setFullscreen", &[serde_json::json!(is_fullscreen())]);
    }
}

static IN_BACKGROUND: AtomicBool = AtomicBool::new(false);

/// While the main window is minimized or hidden to the tray, WebView2 is asked to keep its
/// webviews lean: it drops caches and other memory it can rebuild, and playback carries on.
/// Showing the window again restores the normal level. Only transitions reach WebView2.
pub fn set_main_in_background(app: &AppHandle, background: bool) {
    if IN_BACKGROUND.swap(background, Ordering::Relaxed) == background {
        return;
    }
    #[cfg(windows)]
    for label in [SHELL, ytm::LABEL] {
        let Some(view) = app.get_webview(label) else { continue };
        let _ = view.with_webview(move |w| unsafe {
            use webview2_com::Microsoft::Web::WebView2::Win32::{
                ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
            };
            use windows_core::Interface;
            let level = if background {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
            } else {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
            };
            if let Ok(core) = w.controller().CoreWebView2().and_then(|c| c.cast::<ICoreWebView2_19>()) {
                let _ = core.SetMemoryUsageTargetLevel(level);
            }
        });
    }
}

/// App fullscreen (F11) and fullscreen requested by YTM's video player. The window is
/// fullscreen while either is on, and leaving video fullscreen returns to the app's own
/// state. Kept here rather than read back from the window, which reports the new state only
/// after the resize lands and so would lay out the views for the old one.
static APP_FULLSCREEN: AtomicBool = AtomicBool::new(false);
static VIDEO_FULLSCREEN: AtomicBool = AtomicBool::new(false);
/// The window was maximized when fullscreen began, and is maximized again when it ends.
static MAXIMIZED_BEFORE_FULLSCREEN: AtomicBool = AtomicBool::new(false);

fn is_fullscreen() -> bool {
    APP_FULLSCREEN.load(Ordering::Relaxed) || VIDEO_FULLSCREEN.load(Ordering::Relaxed)
}

/// Positions the YTM view between the title bar and the player bar, and shows it unless
/// it is still loading or the now playing view covers it. Both bars stay in fullscreen, so
/// every control remains reachable: the theme hides YTM's own player bar, and the header
/// swaps its window buttons for an exit button.
pub fn layout_main(app: &AppHandle) {
    let (Some(window), Some(view)) = (app.get_window(MAIN), ytm::webview(app)) else { return };
    let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) else { return };
    let size = size.to_logical::<f64>(scale);
    let ctx = app_ctx(app);
    let video = VIDEO_FULLSCREEN.load(Ordering::Relaxed);
    let owns_header = ctx.memory.read(|m| m.ytm_owns_header);
    let native = native_browse(app);
    let (loading, covered, foreign, mode) =
        ctx.memory.read(|m| (m.ytm_view_loading, (m.now_playing_open || m.tour_open) && !video, m.ytm_foreign_page, m.video_view));
    if native {
        // An animation between video view states places the view itself.
        if ANIMATING.load(Ordering::Relaxed) {
            return;
        }
        let rect = if video {
            Rect { x: 0.0, y: 0.0, w: size.width, h: (size.height - PLAYER_BAR_HEIGHT).max(1.0) }
        } else if covered {
            parked(size)
        } else if foreign {
            content(size)
        } else {
            match mode {
                VideoView::Open => content(size),
                VideoView::Mini => mini(size),
                VideoView::Closed => parked(size),
            }
        };
        place(&view, rect);
        let _ = view.show();
        return;
    }
    let top = if video {
        0.0
    } else if owns_header {
        0.0
    } else {
        TITLE_BAR_HEIGHT
    };
    let bottom = if ctx.settings.get().appearance.modern_ui { PLAYER_BAR_HEIGHT } else { 0.0 };
    let _ = view.set_position(LogicalPosition::new(0.0, top));
    let _ = view.set_size(LogicalSize::new(size.width, (size.height - top - bottom).max(1.0)));
    let _ = if loading || covered { view.hide() } else { view.show() };
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Mini player size and margin, in logical pixels.
const MINI_WIDTH: f64 = 400.0;
const MINI_MARGIN: f64 = 20.0;
const ANIMATION_MS: u64 = 340;

fn content(size: LogicalSize<f64>) -> Rect {
    Rect { x: 0.0, y: NATIVE_TOP_BAR_HEIGHT, w: size.width, h: (size.height - NATIVE_TOP_BAR_HEIGHT - PLAYER_BAR_HEIGHT).max(1.0) }
}

fn mini(size: LogicalSize<f64>) -> Rect {
    let h = MINI_WIDTH * 9.0 / 16.0;
    Rect { x: size.width - MINI_WIDTH - MINI_MARGIN, y: size.height - PLAYER_BAR_HEIGHT - h - MINI_MARGIN, w: MINI_WIDTH, h }
}

/// Just outside the window instead of hidden: YouTube's player loads nothing in a page
/// that has never been visible, so a hidden page would never start playing.
fn parked(size: LogicalSize<f64>) -> Rect {
    Rect { x: size.width + 16.0, ..content(size) }
}

/// Where a view slides from when it opens, and to when it closes: straight down, out of
/// the window.
fn below(r: Rect, size: LogicalSize<f64>) -> Rect {
    Rect { y: size.height + 8.0, ..r }
}

fn place(view: &tauri::Webview, r: Rect) {
    let _ = view.set_position(LogicalPosition::new(r.x.round(), r.y.round()));
    let _ = view.set_size(LogicalSize::new(r.w.round().max(1.0), r.h.round().max(1.0)));
}

static ANIMATING: AtomicBool = AtomicBool::new(false);
static ANIMATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Moves the view between two rectangles, eased, at about 60 frames a second. A newer
/// animation stops an older one. Returns whether it ran to the end.
async fn animate(app: &AppHandle, from: Rect, to: Rect) -> bool {
    let Some(view) = ytm::webview(app) else { return false };
    let generation = ANIMATION.fetch_add(1, Ordering::SeqCst) + 1;
    ANIMATING.store(true, Ordering::Relaxed);
    let _ = view.show();
    let start = std::time::Instant::now();
    let total = std::time::Duration::from_millis(ANIMATION_MS);
    let resize = (from.w - to.w).abs() > 0.5 || (from.h - to.h).abs() > 0.5;
    loop {
        if ANIMATION.load(Ordering::SeqCst) != generation {
            return false;
        }
        let t = (start.elapsed().as_secs_f64() / total.as_secs_f64()).min(1.0);
        // Ease in and out (cubic): a smooth start and a soft landing.
        let e = if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 };
        let lerp = |a: f64, b: f64| a + (b - a) * e;
        let r = Rect { x: lerp(from.x, to.x), y: lerp(from.y, to.y), w: lerp(from.w, to.w), h: lerp(from.h, to.h) };
        if resize {
            place(&view, r);
        } else {
            let _ = view.set_position(LogicalPosition::new(r.x.round(), r.y.round()));
        }
        if t >= 1.0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
    }
    ANIMATING.store(false, Ordering::Relaxed);
    true
}

fn window_size(app: &AppHandle) -> Option<LogicalSize<f64>> {
    let window = app.get_window(MAIN)?;
    Some(window.inner_size().ok()?.to_logical::<f64>(window.scale_factor().ok()?))
}

/// Opens, shrinks or closes the video view with a transition. Closing puts YTM's player
/// page away only after the view has slid out, so the page never jumps underneath.
pub fn set_video_view(app: &AppHandle, to: VideoView) {
    let ctx = app_ctx(app);
    let from_mode = ctx.memory.read(|m| m.video_view);
    if from_mode == to {
        return;
    }
    let Some(size) = window_size(app) else { return };
    let rect = |mode: VideoView| match mode {
        VideoView::Open => content(size),
        VideoView::Mini => mini(size),
        VideoView::Closed => parked(size),
    };
    let from = if from_mode == VideoView::Closed { below(rect(to), size) } else { rect(from_mode) };
    let target = if to == VideoView::Closed { below(rect(from_mode), size) } else { rect(to) };
    ctx.memory.update(app, |m| m.video_view = to);
    // The mini player shows only the video; the full view also its queue and lyrics.
    ytm::call(app, "setMini", &[serde_json::json!(to == VideoView::Mini)]);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if animate(&app, from, target).await {
            if to == VideoView::Closed && app_ctx(&app).memory.read(|m| m.video_view) == VideoView::Closed {
                ytm::call(&app, "closePlayerPage", &[]);
            }
            layout_main(&app);
        }
    });
}

/// Leaving the video view: into the mini player when that is turned on, else closed.
pub fn leave_video_view(app: &AppHandle) {
    let mini = app_ctx(app).settings.get().appearance.video_mini_player;
    let to = if mini && app_ctx(app).memory.read(|m| m.video_view) == VideoView::Open { VideoView::Mini } else { VideoView::Closed };
    set_video_view(app, to);
}

/// The player page button (artwork, Q): opens the video view, enlarges the mini player, or
/// leaves the open view.
pub fn toggle_video_view(app: &AppHandle) {
    match app_ctx(app).memory.read(|m| m.video_view) {
        VideoView::Closed => ytm::call(app, "togglePlayerPage", &[]),
        VideoView::Mini => set_video_view(app, VideoView::Open),
        VideoView::Open => leave_video_view(app),
    }
}

/// The app draws YouTube Music's pages itself. Needs the app's own player bar (Modern UI).
pub fn native_browse(app: &AppHandle) -> bool {
    let s = app_ctx(app).settings.get();
    s.appearance.modern_ui && s.appearance.native_browse
}

fn apply_fullscreen(app: &AppHandle) {
    if let Some(w) = app.get_window(MAIN) {
        let on = is_fullscreen();
        let was = w.is_fullscreen().unwrap_or(false);
        if on && !was {
            // A maximized frameless window keeps its client area at the monitor's work area,
            // even in fullscreen, which leaves the taskbar's strip unpainted (white). Leaving
            // the maximized state first lets fullscreen cover the whole monitor.
            let maximized = w.is_maximized().unwrap_or(false);
            MAXIMIZED_BEFORE_FULLSCREEN.store(maximized, Ordering::Relaxed);
            if maximized {
                let _ = w.unmaximize();
            }
        }
        let _ = w.set_fullscreen(on);
        if !on && was && MAXIMIZED_BEFORE_FULLSCREEN.swap(false, Ordering::Relaxed) {
            let _ = w.maximize();
        }
        layout_main(app);
        emit_window_state(app);
    }
}

/// Video fullscreen, entered and left by YTM's player.
pub fn set_video_fullscreen(app: &AppHandle, fullscreen: bool) {
    VIDEO_FULLSCREEN.store(fullscreen, Ordering::Relaxed);
    apply_fullscreen(app);
}

/// App fullscreen (F11). Leaving it also leaves video fullscreen.
pub fn toggle_fullscreen(app: &AppHandle) {
    let on = !is_fullscreen();
    APP_FULLSCREEN.store(on, Ordering::Relaxed);
    if !on {
        VIDEO_FULLSCREEN.store(false, Ordering::Relaxed);
        ytm::call(app, "exitFullscreen", &[]);
    }
    apply_fullscreen(app);
}

/// Shows or hides the app tour. It covers the YTM view so it can dim the whole window, and
/// finishing it once marks it seen so it is not shown on the next start.
pub fn set_tour_open(app: &AppHandle, open: bool) {
    let ctx = app_ctx(app);
    if !open {
        ctx.state.update(|s| s.tour_seen = true);
    }
    ctx.memory.update(app, |m| m.tour_open = open);
    layout_main(app);
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
            set_main_in_background(app, true);
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
