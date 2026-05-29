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

fn tray_icon() -> Image<'static> {
    const SIZE: u32 = 32;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let border = !(4..28).contains(&x) || !(4..28).contains(&y);
            let key_stem = (8..=13).contains(&x) && (8..=24).contains(&y);
            let key_top = (8..=24).contains(&x) && (8..=13).contains(&y);
            let pixel_on = border || key_stem || key_top;

            if pixel_on {
                let index = ((y * SIZE + x) * 4) as usize;
                rgba[index] = 24;
                rgba[index + 1] = 24;
                rgba[index + 2] = 27;
                rgba[index + 3] = 255;
            }
        }
    }

    Image::new_owned(rgba, SIZE, SIZE)
}

pub fn rebuild_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let menu = build_menu(app)?;
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(&TOOLTIP))?;
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
