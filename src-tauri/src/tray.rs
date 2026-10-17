//! System tray icon and menu.

use crate::settings::TrayIconStyle;
use crate::{app_ctx, windows, ytm};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

const TRAY_ID: &str = "main";

/// The app icon, or a monochrome version of it for White/Black styles.
fn icon(app: &AppHandle) -> Option<Image<'static>> {
    let base = app.default_window_icon()?;
    let shade = match app_ctx(app).settings.get().appearance.tray_icon_style {
        TrayIconStyle::Auto => return Some(base.clone().to_owned()),
        TrayIconStyle::White => 255,
        TrayIconStyle::Black => 0,
    };
    let rgba = base
        .rgba()
        .chunks_exact(4)
        .flat_map(|px| [shade, shade, shade, px[3]])
        .collect::<Vec<u8>>();
    Some(Image::new_owned(rgba, base.width(), base.height()))
}

pub fn refresh_icon(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(icon(app));
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let header = MenuItem::with_id(app, "header", "YTMusic Redux", false, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &header,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "toggle", "Show/Hide Window", true, None::<&str>)?,
            &MenuItem::with_id(app, "playPause", "Play/Pause", true, None::<&str>)?,
            &MenuItem::with_id(app, "previous", "Previous", true, None::<&str>)?,
            &MenuItem::with_id(app, "next", "Next", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "mini", "Mini Player", true, None::<&str>)?,
            &MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("YTMusic Redux")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => windows::toggle_main(app),
            "mini" => {
                let _ = windows::toggle_mini_player(app);
            }
            "settings" => {
                windows::show_main(app);
                let _ = windows::open_settings(app, None);
            }
            "quit" => crate::quit(app),
            other => {
                if let Some(cmd) = ytm::Command::simple(other) {
                    ytm::execute(app, &cmd);
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                windows::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = icon(app) {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}
