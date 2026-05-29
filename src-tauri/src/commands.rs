use crate::{autostart, config, config::Config, state::AppState, toast, translate, tray};
use tauri::{AppHandle, State};

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config()
}

#[tauri::command]
pub fn set_config(cfg: Config, state: State<'_, AppState>) -> CommandResult<()> {
    config::save_config(&cfg).map_err(|error| error.to_string())?;
    state.set_config(cfg);
    Ok(())
}

#[tauri::command]
pub fn pause_caps_lock(
    paused: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    state.set_caps_paused(paused);
    tray::rebuild_tray_menu(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn is_caps_paused(state: State<'_, AppState>) -> bool {
    state.caps_paused()
}

#[tauri::command]
pub fn set_auto_start(enabled: bool) -> CommandResult<()> {
    autostart::set_auto_start(enabled).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn is_auto_start() -> CommandResult<bool> {
    autostart::is_auto_start().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn test_translate_api(
    server_url: String,
    api_key: String,
    target: String,
) -> CommandResult<String> {
    translate::test_translate_api(&server_url, &api_key, &target)
}

#[tauri::command]
pub fn replace_with_translation(text: String) -> CommandResult<()> {
    translate::replace_with_translation(text)
}

#[tauri::command]
pub fn copy_to_clipboard(text: String) -> CommandResult<()> {
    translate::copy_to_clipboard(text)
}

#[tauri::command]
pub fn hide_translation_toast() {
    toast::hide_translation_toast();
}
