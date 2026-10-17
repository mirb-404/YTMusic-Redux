//! User stylesheet injected into YouTube Music, reloaded live when the file changes.

use crate::{app_ctx, ytm};
use notify::{EventKind, RecursiveMode, Watcher};
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::AppHandle;

fn css_path(app: &AppHandle) -> Option<PathBuf> {
    let settings = app_ctx(app).settings.get();
    settings
        .appearance
        .custom_css_enabled
        .then(|| settings.appearance.custom_css_path.clone())
        .flatten()
        .map(PathBuf::from)
}

/// The stylesheet that should currently be applied, if any.
pub fn current_css(app: &AppHandle) -> Option<String> {
    std::fs::read_to_string(css_path(app)?).ok()
}

fn push(app: &AppHandle) {
    let css = current_css(app).map_or(Value::Null, |c| json!(c));
    ytm::call(app, "setCustomCss", &[css]);
}

/// Applies the current settings: injects or removes the CSS and (re)starts the file watcher.
pub fn refresh(app: &AppHandle) {
    let ctx = app_ctx(app);
    let mut watcher_slot = ctx.css_watcher.lock().unwrap();
    *watcher_slot = None;
    push(app);
    let Some(path) = css_path(app) else { return };
    let watch_app = app.clone();
    let file_name = path.file_name().map(|n| n.to_os_string());
    let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event {
            let ours = event.paths.iter().any(|p| p.file_name().map(|n| n.to_os_string()) == file_name);
            if ours && matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)) {
                push(&watch_app);
            }
        }
    });
    match watcher {
        Ok(mut w) => {
            // Watch the directory so editors that replace the file on save keep working.
            let target = path.parent().map(PathBuf::from).unwrap_or(path);
            if let Err(e) = w.watch(&target, RecursiveMode::NonRecursive) {
                log::warn!("Cannot watch custom CSS: {e}");
            }
            *watcher_slot = Some(w);
        }
        Err(e) => log::warn!("Cannot create CSS watcher: {e}"),
    }
}
