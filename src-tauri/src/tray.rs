use crate::{state::AppState, window};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Runtime,
};

const TRAY_ID: &str = "main";
const MENU_SETTINGS_ID: &str = "settings";
const MENU_PAUSE_ID: &str = "pause_caps";
const MENU_EXIT_ID: &str = "exit";
const TOOLTIP: &str = "KeyTweak";

fn is_system_dark_theme() -> bool {
    use windows::Win32::System::Registry::*;

    unsafe {
        let mut h_key: HKEY = Default::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            0,
            KEY_READ,
            &mut h_key,
        )
        .is_err()
        {
            return false;
        }

        let mut data: u32 = 1;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
        let result = RegQueryValueExW(
            h_key,
            windows::core::w!("SystemUsesLightTheme"),
            None,
            None,
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut data_size),
        );

        let _ = RegCloseKey(h_key);

        result.is_ok() && data == 0
    }
}

pub fn tray_icon() -> Image<'static> {
    if is_system_dark_theme() {
        Image::from_bytes(include_bytes!("../icons/icon-light.ico"))
    } else {
        Image::from_bytes(include_bytes!("../icons/icon.ico"))
    }
    .expect("failed to load tray icon")
}

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_icon())
        .tooltip(TOOLTIP)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SETTINGS_ID => {
                let _ = window::show_settings(app);
            }
            MENU_PAUSE_ID => {
                if let Some(state) = app.try_state::<AppState>() {
                    let paused = !state.caps_paused();
                    state.set_caps_paused(paused);
                    let _ = rebuild_tray_menu(app);
                }
            }
            MENU_EXIT_ID => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

pub fn rebuild_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let menu = build_menu(app)?;
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(&TOOLTIP))?;
        tray.set_icon(Some(tray_icon()))?;
    }

    Ok(())
}

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let paused = app
        .try_state::<AppState>()
        .map(|state| state.caps_paused())
        .unwrap_or(false);
    let pause_text = if paused {
        "Возобновить переключение Caps Lock"
    } else {
        "Приостановить переключение Caps Lock"
    };

    let settings = MenuItem::with_id(app, MENU_SETTINGS_ID, "Настройки...", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, MENU_PAUSE_ID, pause_text, true, None::<&str>)?;
    let exit = MenuItem::with_id(app, MENU_EXIT_ID, "Выход", true, None::<&str>)?;

    Menu::with_items(app, &[&settings, &pause, &exit])
}
