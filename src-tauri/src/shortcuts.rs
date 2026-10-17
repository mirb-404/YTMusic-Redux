//! Global keyboard shortcuts.

use crate::settings::Shortcuts;
use crate::{app_ctx, ytm};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Re-registers every configured shortcut and records which ones failed.
pub fn register(app: &AppHandle, shortcuts: &Shortcuts) {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();
    let mut failed = std::collections::BTreeMap::new();
    for (key, accelerator, command) in shortcuts.bindings() {
        if accelerator.is_empty() {
            failed.insert(key.to_string(), false);
            continue;
        }
        let Some(cmd) = ytm::Command::simple(command) else { continue };
        let result = manager.on_shortcut(accelerator, move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                ytm::execute(app, &cmd);
            }
        });
        if let Err(e) = &result {
            log::info!("Failed to register shortcut {key} ({accelerator}): {e}");
        }
        failed.insert(key.to_string(), result.is_err());
    }
    app_ctx(app).memory.update(app, |m| m.shortcut_register_failed = failed);
}
