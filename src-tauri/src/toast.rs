use serde::Serialize;
use std::{
    sync::OnceLock,
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Wry};
use windows::Win32::{
    Foundation::POINT,
    UI::WindowsAndMessaging::GetCursorPos,
};

const TOAST_LABEL: &str = "toast";
const TOAST_WIDTH: u32 = 360;
const STARTUP_TOAST_HEIGHT: u32 = 104;
const TRANSLATION_TOAST_HEIGHT: u32 = 220;

static APP_HANDLE: OnceLock<AppHandle<Wry>> = OnceLock::new();

pub fn set_app_handle(app: AppHandle<Wry>) {
    let _ = APP_HANDLE.set(app);
}

#[derive(Debug, Clone, Serialize)]
pub struct TranslationToastPayload {
    pub original: String,
    pub translated: String,
    pub source_lang: String,
    pub target_lang: String,
    pub reverse: bool,
}

#[derive(Debug, Clone, Serialize)]
struct AppToastPayload {
    title: String,
    message: String,
}

struct MonitorBounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl MonitorBounds {
    fn from_monitor(monitor: &tauri::Monitor) -> Self {
        let pos = monitor.position();
        let size = monitor.size();
        Self {
            left: pos.x,
            top: pos.y,
            right: pos.x + size.width as i32,
            bottom: pos.y + size.height as i32,
        }
    }

    fn center(&self) -> PhysicalPosition<i32> {
        PhysicalPosition::new((self.left + self.right) / 2, (self.top + self.bottom) / 2)
    }

    fn corner_pos(&self, win_size: tauri::PhysicalSize<u32>, margin: i32) -> PhysicalPosition<i32> {
        PhysicalPosition::new(
            self.right - win_size.width as i32 - margin,
            self.bottom - win_size.height as i32 - margin,
        )
    }
}

pub fn show_startup_toast() {
    let Some(app) = APP_HANDLE.get() else {
        return;
    };
    let app = app.clone();

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(700));

        let Some(window) = app.get_webview_window(TOAST_LABEL) else {
            return;
        };

        let _ = show_toast(&window, STARTUP_TOAST_HEIGHT, "show-app-toast", AppToastPayload {
            title: "KeyTweak запущен".to_string(),
            message: "Горячие клавиши и Caps Lock работают в фоне.".to_string(),
        }, position_startup_toast_window);
    });
}

pub fn show_translation_toast(payload: TranslationToastPayload) {
    let Some(app) = APP_HANDLE.get() else {
        return;
    };
    let Some(window) = app.get_webview_window(TOAST_LABEL) else {
        return;
    };

    let _ = show_toast(&window, TRANSLATION_TOAST_HEIGHT, "show-translation", payload, position_toast_window);
    let _ = window.set_focus();
}

pub fn show_translation_error(message: &str) {
    show_translation_toast(TranslationToastPayload {
        original: String::new(),
        translated: message.to_string(),
        source_lang: String::new(),
        target_lang: String::new(),
        reverse: false,
    });
}

pub fn hide_translation_toast() {
    if let Some(app) = APP_HANDLE.get() {
        if let Some(window) = app.get_webview_window(TOAST_LABEL) {
            let _ = window.hide();
        }
    }
}

fn show_toast(
    window: &tauri::WebviewWindow<Wry>,
    height: u32,
    event: &str,
    payload: impl Serialize + Clone,
    position: fn(&tauri::WebviewWindow<Wry>) -> tauri::Result<()>,
) -> tauri::Result<()> {
    window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(TOAST_WIDTH, height)))?;
    position(window)?;
    window.emit(event, payload)?;
    window.show()
}

fn position_toast_window(window: &tauri::WebviewWindow<Wry>) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let mb = MonitorBounds::from_monitor(&monitor);
    let window_size = window.outer_size()?;

    let cursor = cursor_position().unwrap_or_else(|| mb.center());

    const OFFSET_X: i32 = 16;
    const OFFSET_Y: i32 = 16;

    let win_w = window_size.width as i32;
    let win_h = window_size.height as i32;

    let mut x = cursor.x + OFFSET_X;
    let mut y = cursor.y + OFFSET_Y;

    if x + win_w > mb.right {
        x = cursor.x - OFFSET_X - win_w;
    }
    if y + win_h > mb.bottom {
        y = cursor.y - OFFSET_Y - win_h;
    }

    let margin = 8;
    if x < mb.left + margin {
        x = mb.left + margin;
    }
    if y < mb.top + margin {
        y = mb.top + margin;
    }
    if x + win_w > mb.right - margin {
        x = mb.right - margin - win_w;
    }
    if y + win_h > mb.bottom - margin {
        y = mb.bottom - margin - win_h;
    }

    window.set_position(PhysicalPosition::new(x, y))
}

fn position_startup_toast_window(window: &tauri::WebviewWindow<Wry>) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let mb = MonitorBounds::from_monitor(&monitor);
    let window_size = window.outer_size()?;
    let margin = 18;

    window.set_position(mb.corner_pos(window_size, margin))
}

fn cursor_position() -> Option<PhysicalPosition<i32>> {
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point).ok()?;
    }
    Some(PhysicalPosition::new(point.x, point.y))
}
