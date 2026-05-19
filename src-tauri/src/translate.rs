use crate::{config::TranslateConfig, keyboard_hook::ModifierState};
use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::{
    mem::size_of,
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Wry};
use windows::Win32::{
    Foundation::POINT,
    UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
            VK_CONTROL,
        },
        WindowsAndMessaging::GetCursorPos,
    },
};

const C_KEY: u32 = 0x43;
const V_KEY: u32 = 0x56;
const DOUBLE_C_WINDOW: Duration = Duration::from_millis(500);
const CLIPBOARD_SETTLE_DELAY: Duration = Duration::from_millis(120);
const TOAST_LABEL: &str = "toast";

#[derive(Debug, Clone)]
struct RuntimeConfig {
    server_url: String,
    api_key: String,
    target_language: String,
    hotkey_translate: String,
    hotkey_reverse: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::from_config(&TranslateConfig::default())
    }
}

impl RuntimeConfig {
    fn from_config(config: &TranslateConfig) -> Self {
        Self {
            server_url: config.server_url.clone(),
            api_key: config.api_key.clone(),
            target_language: config.target_language.clone(),
            hotkey_translate: config.hotkey_translate.clone(),
            hotkey_reverse: config.hotkey_reverse.clone(),
        }
    }
}

#[derive(Debug, Default)]
struct CtrlCState {
    last_press: Option<Instant>,
    clipboard_backup: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct TranslationToastPayload {
    original: String,
    translated: String,
    target_lang: String,
    reverse: bool,
}

#[derive(Debug, Serialize)]
struct LibreTranslateRequest<'a> {
    q: &'a str,
    source: &'a str,
    target: &'a str,
    format: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct LibreTranslateResponse {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

#[derive(Debug, Deserialize)]
struct LibreTranslateError {
    error: String,
}

static CONFIG: OnceLock<Mutex<RuntimeConfig>> = OnceLock::new();
static CTRL_C_STATE: OnceLock<Mutex<CtrlCState>> = OnceLock::new();
static APP_HANDLE: OnceLock<AppHandle<Wry>> = OnceLock::new();

pub fn configure(config: &TranslateConfig) {
    let mut runtime_config = config_store()
        .lock()
        .expect("translate config mutex poisoned");
    *runtime_config = RuntimeConfig::from_config(config);
}

pub fn set_app_handle(app: AppHandle<Wry>) {
    let _ = APP_HANDLE.set(app);
}

pub fn handle_keydown(vk_code: u32, modifiers: ModifierState) -> bool {
    if vk_code != C_KEY {
        if !modifiers.ctrl {
            reset_ctrl_c_state();
        }
        return false;
    }

    let config = current_config();

    if modifiers.ctrl && modifiers.shift && !modifiers.alt && !modifiers.win {
        if config.hotkey_reverse.eq_ignore_ascii_case("ctrl+shift+c") {
            trigger_reverse_translate(config);
            return true;
        }
        return false;
    }

    if modifiers.ctrl
        && !modifiers.shift
        && !modifiers.alt
        && !modifiers.win
        && config.hotkey_translate.eq_ignore_ascii_case("ctrl+c+c")
    {
        return handle_ctrl_c_for_translate();
    }

    false
}

fn handle_ctrl_c_for_translate() -> bool {
    let now = Instant::now();
    let mut state = ctrl_c_state()
        .lock()
        .expect("translate ctrl+c mutex poisoned");

    let is_double_press = state
        .last_press
        .is_some_and(|last| now.duration_since(last) <= DOUBLE_C_WINDOW);

    if is_double_press {
        let backup = state.clipboard_backup.take();
        state.last_press = None;
        drop(state);

        trigger_translate_from_existing_copy(backup);
        true
    } else {
        state.last_press = Some(now);
        state.clipboard_backup = current_clipboard_text();
        false
    }
}

fn trigger_translate_from_existing_copy(clipboard_backup: Option<String>) {
    let config = current_config();
    thread::spawn(move || {
        thread::sleep(CLIPBOARD_SETTLE_DELAY);
        let Some(text) = current_clipboard_text().filter(|text| !text.trim().is_empty()) else {
            restore_clipboard_text(clipboard_backup);
            return;
        };

        restore_clipboard_text(clipboard_backup);
        run_translation_flow(text, config, false);
    });
}

fn trigger_reverse_translate(config: RuntimeConfig) {
    let clipboard_backup = current_clipboard_text();

    thread::spawn(move || {
        send_ctrl_c();
        thread::sleep(CLIPBOARD_SETTLE_DELAY);

        let Some(text) = current_clipboard_text().filter(|text| !text.trim().is_empty()) else {
            restore_clipboard_text(clipboard_backup);
            return;
        };

        restore_clipboard_text(clipboard_backup);
        run_translation_flow(text, config, true);
    });
}

fn run_translation_flow(text: String, config: RuntimeConfig, reverse: bool) {
    let server_url = config.server_url.trim();
    if server_url.is_empty() {
        show_translation_error("Ошибка перевода: адрес сервера LibreTranslate не настроен.");
        return;
    }

    let detected = detect_language(&text);
    let target = if reverse {
        reverse_target_language(&config.target_language)
    } else {
        // Auto: if source is Russian, translate to English; otherwise to Russian.
        match detected {
            "ru" => "en".to_string(),
            _ => "ru".to_string(),
        }
    };

    match translate_text(&text, &target, server_url, &config.api_key) {
        Ok(translated) => show_translation_toast(TranslationToastPayload {
            original: text,
            translated,
            target_lang: target,
            reverse,
        }),
        Err(error) => show_translation_error(&format!("Ошибка перевода: {error}")),
    }
}

/// Lightweight language detection: returns "ru" if the text contains
/// any Cyrillic letter, otherwise "en". Good enough for a binary RU/EN choice.
fn detect_language(text: &str) -> &'static str {
    if text.chars().any(|c| matches!(c, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}')) {
        "ru"
    } else {
        "en"
    }
}

fn translate_text(
    text: &str,
    target: &str,
    server_url: &str,
    api_key: &str,
) -> Result<String, String> {
    let endpoint = format!("{}/translate", server_url.trim_end_matches('/'));
    let api_key = api_key.trim();
    let request = LibreTranslateRequest {
        q: text,
        source: "auto",
        target,
        format: "text",
        api_key: if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        },
    };
    let response = reqwest::blocking::Client::new()
        .post(&endpoint)
        .json(&request)
        .send()
        .map_err(|error| format!("сетевая ошибка ({error})"))?;

    let status = response.status();
    if !status.is_success() {
        // Try to extract LibreTranslate error message from body.
        let body_text = response.text().unwrap_or_default();
        let detail = serde_json::from_str::<LibreTranslateError>(&body_text)
            .map(|e| e.error)
            .unwrap_or_else(|_| body_text);

        return Err(match status.as_u16() {
            400 => format!("ошибочный запрос: {detail}"),
            401 | 403 => "неверный или отсутствующий API-ключ".to_string(),
            429 => "превышен лимит запросов".to_string(),
            500 => format!("ошибка сервера LibreTranslate: {detail}"),
            code => format!("LibreTranslate вернул HTTP {code}: {detail}"),
        });
    }

    let response: LibreTranslateResponse = response
        .json()
        .map_err(|error| format!("неверный ответ API ({error})"))?;

    Ok(response.translated_text)
}

pub fn test_translate_api(
    server_url: &str,
    api_key: &str,
    target: &str,
) -> Result<String, String> {
    let server_url = server_url.trim();
    if server_url.is_empty() {
        return Err("Адрес сервера LibreTranslate не настроен".to_string());
    }

    let target = target.trim().to_lowercase();
    if target != "ru" && target != "en" {
        return Err("целевой язык должен быть 'ru' или 'en'".to_string());
    }

    translate_text("hello", &target, server_url, api_key)
}

/// Replaces the currently selected text in the active window with `text`
/// by hiding the toast, putting `text` on the clipboard, sending Ctrl+V,
/// and restoring the previous clipboard contents.
pub fn replace_with_translation(text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("нечего вставлять".to_string());
    }

    // Hide toast first so focus returns to the previous (target) window.
    if let Some(app) = APP_HANDLE.get() {
        if let Some(window) = app.get_webview_window(TOAST_LABEL) {
            let _ = window.hide();
        }
    }

    // Run the rest off the main thread so the IPC call returns immediately
    // and the OS has time to refocus the previous foreground window.
    thread::spawn(move || {
        let backup = current_clipboard_text();

        if let Ok(mut clipboard) = Clipboard::new() {
            if clipboard.set_text(&text).is_err() {
                return;
            }
        } else {
            return;
        }

        // Give the clipboard and the OS a moment to settle and refocus.
        thread::sleep(CLIPBOARD_SETTLE_DELAY);
        send_ctrl_v();

        // Wait for the target app to consume the paste before restoring the clipboard.
        thread::sleep(Duration::from_millis(200));
        restore_clipboard_text(backup);
    });

    Ok(())
}

fn show_translation_toast(payload: TranslationToastPayload) {
    let Some(app) = APP_HANDLE.get() else {
        return;
    };
    let Some(window) = app.get_webview_window(TOAST_LABEL) else {
        return;
    };

    let _ = position_toast_window(&window);
    let _ = window.emit("show-translation", payload);
    let _ = window.show();
    let _ = window.set_focus();
}

fn show_translation_error(message: &str) {
    show_translation_toast(TranslationToastPayload {
        original: String::new(),
        translated: message.to_string(),
        target_lang: String::new(),
        reverse: false,
    });
}

fn position_toast_window(window: &tauri::WebviewWindow<Wry>) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let window_size = window.outer_size()?;

    let cursor = cursor_position().unwrap_or_else(|| PhysicalPosition::new(
        monitor_pos.x + monitor_size.width as i32 / 2,
        monitor_pos.y + monitor_size.height as i32 / 2,
    ));

    // Default offset: place the toast slightly below-right of the cursor.
    const OFFSET_X: i32 = 16;
    const OFFSET_Y: i32 = 16;

    let win_w = window_size.width as i32;
    let win_h = window_size.height as i32;

    let mut x = cursor.x + OFFSET_X;
    let mut y = cursor.y + OFFSET_Y;

    let monitor_left = monitor_pos.x;
    let monitor_top = monitor_pos.y;
    let monitor_right = monitor_pos.x + monitor_size.width as i32;
    let monitor_bottom = monitor_pos.y + monitor_size.height as i32;

    // If the toast would overflow the right edge, flip to the left of the cursor.
    if x + win_w > monitor_right {
        x = cursor.x - OFFSET_X - win_w;
    }
    // If the toast would overflow the bottom edge, flip above the cursor.
    if y + win_h > monitor_bottom {
        y = cursor.y - OFFSET_Y - win_h;
    }

    // Clamp to monitor bounds in case both flips still leave it off-screen.
    let margin = 8;
    if x < monitor_left + margin {
        x = monitor_left + margin;
    }
    if y < monitor_top + margin {
        y = monitor_top + margin;
    }
    if x + win_w > monitor_right - margin {
        x = monitor_right - margin - win_w;
    }
    if y + win_h > monitor_bottom - margin {
        y = monitor_bottom - margin - win_h;
    }

    window.set_position(PhysicalPosition::new(x, y))
}

fn cursor_position() -> Option<PhysicalPosition<i32>> {
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point).ok()?;
    }
    Some(PhysicalPosition::new(point.x, point.y))
}

fn send_ctrl_c() {
    let inputs = [
        vk_input(VK_CONTROL, false),
        vk_input(VIRTUAL_KEY(C_KEY as u16), false),
        vk_input(VIRTUAL_KEY(C_KEY as u16), true),
        vk_input(VK_CONTROL, true),
    ];

    unsafe {
        let _ = SendInput(&inputs, size_of::<INPUT>() as i32);
    }
}

fn send_ctrl_v() {
    let inputs = [
        vk_input(VK_CONTROL, false),
        vk_input(VIRTUAL_KEY(V_KEY as u16), false),
        vk_input(VIRTUAL_KEY(V_KEY as u16), true),
        vk_input(VK_CONTROL, true),
    ];

    unsafe {
        let _ = SendInput(&inputs, size_of::<INPUT>() as i32);
    }
}

fn vk_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn current_clipboard_text() -> Option<String> {
    Clipboard::new().ok()?.get_text().ok()
}

fn restore_clipboard_text(text: Option<String>) {
    if let Some(text) = text {
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }
}

fn reverse_target_language(target_language: &str) -> String {
    if target_language.eq_ignore_ascii_case("ru") {
        "en".to_string()
    } else {
        "ru".to_string()
    }
}

fn reset_ctrl_c_state() {
    if let Ok(mut state) = ctrl_c_state().lock() {
        state.last_press = None;
        state.clipboard_backup = None;
    }
}

fn current_config() -> RuntimeConfig {
    config_store()
        .lock()
        .expect("translate config mutex poisoned")
        .clone()
}

fn config_store() -> &'static Mutex<RuntimeConfig> {
    CONFIG.get_or_init(|| Mutex::new(RuntimeConfig::default()))
}

fn ctrl_c_state() -> &'static Mutex<CtrlCState> {
    CTRL_C_STATE.get_or_init(|| Mutex::new(CtrlCState::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_target_switches_between_ru_and_en() {
        assert_eq!(reverse_target_language("ru"), "en");
        assert_eq!(reverse_target_language("RU"), "en");
        assert_eq!(reverse_target_language("en"), "ru");
    }

    #[test]
    fn detect_language_recognizes_cyrillic() {
        assert_eq!(detect_language("Привет"), "ru");
        assert_eq!(detect_language("Hello"), "en");
        assert_eq!(detect_language("Hello, мир!"), "ru");
        assert_eq!(detect_language(""), "en");
        assert_eq!(detect_language("123 !@#"), "en");
    }
}
