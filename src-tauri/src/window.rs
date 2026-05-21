use tauri::{AppHandle, Manager, Runtime};

pub const SETTINGS_WINDOW_LABEL: &str = "settings";

pub fn show_settings<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        window.set_skip_taskbar(false)?;
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
    }

    Ok(())
}
