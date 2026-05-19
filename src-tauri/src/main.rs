mod autoreplace;
#[allow(dead_code)]
mod autostart;
mod capslock;
mod commands;
mod config;
mod keyboard_hook;
mod state;
mod translate;
mod tray;
mod window;

use crate::{config::Config, state::AppState};
use tauri::Manager;

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = window::show_settings(app);
        }))
        .setup(|app| {
            let config = config::load_config().unwrap_or_else(|error| {
                log::error!("failed to load config, using defaults: {error}");
                Config::default()
            });

            let state = AppState::new(config);
            translate::set_app_handle(app.handle().clone());
            state.install_keyboard_hook()?;

            app.manage(state);
            tray::setup_tray(&app.handle())?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == window::SETTINGS_WINDOW_LABEL {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config,
            commands::pause_caps_lock,
            commands::is_caps_paused,
            commands::set_auto_start,
            commands::is_auto_start,
            commands::test_translate_api,
            commands::replace_with_translation,
        ])
        .build(tauri::generate_context!())
        .expect("error while building KeyTweak")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app.try_state::<AppState>() {
                    state.uninstall_keyboard_hook();
                }
            }
        });
}
