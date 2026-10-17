//! Taskbar integration: track progress and (Windows) thumbnail toolbar buttons.

use crate::{app_ctx, windows as app_windows};
use tauri::window::{ProgressBarState, ProgressBarStatus};
use tauri::{AppHandle, Manager};

pub fn clear_progress(app: &AppHandle) {
    if let Some(w) = app.get_window(app_windows::MAIN) {
        let _ = w.set_progress_bar(ProgressBarState { status: Some(ProgressBarStatus::None), progress: None });
    }
}

/// Follows the player state for the lifetime of the app.
pub async fn run(app: AppHandle) {
    let mut rx = app_ctx(&app).player.subscribe();
    let mut last_buttons: Option<(bool, bool)> = None;
    while rx.changed().await.is_ok() {
        let state = rx.borrow_and_update().clone();
        let has_video = state.video.is_some();
        let playing = state.is_playing();

        if app_ctx(&app).settings.get().playback.progress_in_taskbar {
            if let Some(w) = app.get_window(app_windows::MAIN) {
                let duration = state.video.as_ref().map_or(0, |v| v.duration_seconds);
                let progress = if duration > 0 { (state.player.video_progress / duration as f64 * 100.0).clamp(0.0, 100.0) as u64 } else { 0 };
                let status = match (has_video, playing) {
                    (false, _) => ProgressBarStatus::None,
                    (true, true) => ProgressBarStatus::Normal,
                    (true, false) => ProgressBarStatus::Paused,
                };
                let _ = w.set_progress_bar(ProgressBarState { status: Some(status), progress: Some(progress) });
            }
        }

        if last_buttons != Some((has_video, playing)) {
            last_buttons = Some((has_video, playing));
            #[cfg(windows)]
            thumbbar::update(&app, has_video, playing);
        }
    }
}

#[cfg(windows)]
mod thumbbar {
    //! ITaskbarList3 thumbnail toolbar (Previous / Play-Pause / Next).

    use crate::{windows as app_windows, ytm};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;
    use tauri::{AppHandle, Manager};
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::{
        DefSubclassProc, ITaskbarList3, SetWindowSubclass, TaskbarList, THBF_DISABLED, THBF_ENABLED, THBN_CLICKED, THB_FLAGS,
        THB_ICON, THB_TOOLTIP, THUMBBUTTON,
    };
    use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO, WM_COMMAND};

    const ID_PREVIOUS: u32 = 1;
    const ID_PLAY_PAUSE: u32 = 2;
    const ID_NEXT: u32 = 3;
    const SIZE: usize = 32;

    static APP: OnceLock<AppHandle> = OnceLock::new();
    static ADDED: AtomicBool = AtomicBool::new(false);

    #[derive(Clone, Copy)]
    enum Glyph {
        Previous,
        Play,
        Pause,
        Next,
    }

    fn in_triangle(x: f32, y: f32, (ax, ay): (f32, f32), (bx, by): (f32, f32), (cx, cy): (f32, f32)) -> bool {
        let d = |px: f32, py: f32, qx: f32, qy: f32, rx: f32, ry: f32| (px - rx) * (qy - ry) - (qx - rx) * (py - ry);
        let (d1, d2, d3) = (d(x, y, ax, ay, bx, by), d(x, y, bx, by, cx, cy), d(x, y, cx, cy, ax, ay));
        !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
    }

    /// Coverage of a glyph at a point in a 0..1 unit square.
    fn covers(glyph: Glyph, x: f32, y: f32) -> bool {
        let bar = |x0: f32, x1: f32| x >= x0 && x <= x1 && (0.22..=0.78).contains(&y);
        match glyph {
            Glyph::Play => in_triangle(x, y, (0.3, 0.2), (0.3, 0.8), (0.8, 0.5)),
            Glyph::Pause => bar(0.28, 0.43) || bar(0.57, 0.72),
            Glyph::Next => in_triangle(x, y, (0.22, 0.22), (0.22, 0.78), (0.64, 0.5)) || bar(0.66, 0.78),
            Glyph::Previous => in_triangle(x, y, (0.78, 0.22), (0.78, 0.78), (0.36, 0.5)) || bar(0.22, 0.34),
        }
    }

    /// Rasterizes a white glyph (4x4 supersampled) into a premultiplied BGRA icon.
    fn make_icon(glyph: Glyph) -> Option<HICON> {
        let mut bgra = vec![0u8; SIZE * SIZE * 4];
        for py in 0..SIZE {
            for px in 0..SIZE {
                let mut hits = 0u32;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let x = (px as f32 + (sx as f32 + 0.5) / 4.0) / SIZE as f32;
                        let y = (py as f32 + (sy as f32 + 0.5) / 4.0) / SIZE as f32;
                        hits += covers(glyph, x, y) as u32;
                    }
                }
                let a = (hits * 255 / 16) as u8;
                let i = (py * SIZE + px) * 4;
                bgra[i..i + 4].copy_from_slice(&[a, a, a, a]);
            }
        }
        unsafe {
            let color = CreateBitmap(SIZE as i32, SIZE as i32, 1, 32, Some(bgra.as_ptr().cast()));
            let mask_bits = vec![0u8; SIZE * SIZE / 8];
            let mask = CreateBitmap(SIZE as i32, SIZE as i32, 1, 1, Some(mask_bits.as_ptr().cast()));
            let info = ICONINFO { fIcon: BOOL(1), xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
            let icon = CreateIconIndirect(&info).ok();
            let _ = DeleteObject(color.into());
            let _ = DeleteObject(mask.into());
            icon
        }
    }

    fn icons() -> &'static [Option<HICON>; 4] {
        struct Icons([Option<HICON>; 4]);
        // SAFETY: icon handles are process-wide GDI handles, only used on the UI thread.
        unsafe impl Send for Icons {}
        unsafe impl Sync for Icons {}
        static ICONS: OnceLock<Icons> = OnceLock::new();
        &ICONS
            .get_or_init(|| Icons([Glyph::Previous, Glyph::Play, Glyph::Pause, Glyph::Next].map(make_icon)))
            .0
    }

    fn button(id: u32, icon: Option<HICON>, tip: &str, enabled: bool) -> THUMBBUTTON {
        let mut sz_tip = [0u16; 260];
        for (dst, src) in sz_tip.iter_mut().zip(tip.encode_utf16()) {
            *dst = src;
        }
        THUMBBUTTON {
            dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
            iId: id,
            iBitmap: 0,
            hIcon: icon.unwrap_or_default(),
            szTip: sz_tip,
            dwFlags: if enabled { THBF_ENABLED } else { THBF_DISABLED },
        }
    }

    unsafe extern "system" fn subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
        if msg == WM_COMMAND && ((wparam.0 >> 16) & 0xffff) as u32 == THBN_CLICKED {
            if let Some(app) = APP.get() {
                let cmd = match (wparam.0 & 0xffff) as u32 {
                    ID_PREVIOUS => Some(ytm::Command::Previous),
                    ID_PLAY_PAUSE => Some(ytm::Command::PlayPause),
                    ID_NEXT => Some(ytm::Command::Next),
                    _ => None,
                };
                if let Some(cmd) = cmd {
                    ytm::execute(app, &cmd);
                }
            }
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    pub fn update(app: &AppHandle, has_video: bool, playing: bool) {
        let app = app.clone();
        let _ = app.clone().run_on_main_thread(move || {
            let Some(window) = app.get_window(app_windows::MAIN) else { return };
            let Ok(tauri_hwnd) = window.hwnd() else { return };
            let hwnd = HWND(tauri_hwnd.0);
            let icons = icons();
            let buttons = [
                button(ID_PREVIOUS, icons[0], "Previous", has_video),
                button(ID_PLAY_PAUSE, if playing { icons[2] } else { icons[1] }, if playing { "Pause" } else { "Play" }, has_video),
                button(ID_NEXT, icons[3], "Next", has_video),
            ];
            unsafe {
                let Ok(taskbar) = CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER) else { return };
                if taskbar.HrInit().is_err() {
                    return;
                }
                if !ADDED.load(Ordering::SeqCst) {
                    let _ = APP.set(app.clone());
                    let _ = SetWindowSubclass(hwnd, Some(subclass_proc), 0x5954_4d52, 0);
                    if taskbar.ThumbBarAddButtons(hwnd, &buttons).is_ok() {
                        ADDED.store(true, Ordering::SeqCst);
                    }
                } else {
                    let _ = taskbar.ThumbBarUpdateButtons(hwnd, &buttons);
                }
            }
        });
    }
}
