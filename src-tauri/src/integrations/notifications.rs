//! Native "now playing" notification on song change.

use crate::app_ctx;
use crate::player::Thumbnail;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

/// Smallest thumbnail that is still larger than 100x100 (port of upstream heuristic).
fn notification_thumbnail(thumbnails: &[Thumbnail]) -> Option<&Thumbnail> {
    thumbnails
        .iter()
        .filter(|t| t.width > 100 && t.height > 100 && t.width < 1024 && t.height < 1024)
        .min_by_key(|t| t.width * t.height)
}

async fn download_icon(app: &AppHandle, url: &str) -> Option<String> {
    let bytes = reqwest::get(url).await.ok()?.error_for_status().ok()?.bytes().await.ok()?;
    let path = app.path().app_cache_dir().ok()?.join("notification-art.jpg");
    std::fs::create_dir_all(path.parent()?).ok()?;
    std::fs::write(&path, &bytes).ok()?;
    Some(path.to_string_lossy().into_owned())
}

pub async fn run(app: AppHandle) {
    let mut rx = app_ctx(&app).player.subscribe();
    let mut last_id = String::new();
    while rx.changed().await.is_ok() {
        let state = rx.borrow_and_update().clone();
        let Some(video) = state.video.as_ref().filter(|_| state.is_playing()) else { continue };
        if video.id == last_id {
            continue;
        }
        last_id = video.id.clone();
        let icon = match notification_thumbnail(&video.thumbnails) {
            Some(t) => download_icon(&app, &t.url).await,
            None => None,
        };
        let mut builder = app.notification().builder().title(&video.title).body(&video.author).silent();
        if let Some(icon) = icon {
            builder = builder.icon(icon);
        }
        if let Err(e) = builder.show() {
            log::warn!("Notification failed: {e}");
        }
    }
}
