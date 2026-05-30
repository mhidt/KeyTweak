use crate::{config::{TranslateConfig, ModuleId}, exclusions, keyboard_hook::ModifierState, keys::{self, press, press_code}, toast::{TranslationToastPayload, hide_translation_toast, show_translation_error, show_translation_toast}};
use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use windows::Win32::{
    UI::{
        Input::KeyboardAndMouse::{
            VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_SHIFT,
        },
    },
};

const C_KEY: u32 = 0x43;
const DOUBLE_C_WINDOW: Duration = Duration::from_millis(500);
const CLIPBOARD_SETTLE_DELAY: Duration = Duration::from_millis(120);

/// A parsed hotkey definition.
/// Examples:
///   "ctrl+alt+r"   -> modifiers={ctrl,alt}, key=0x52, repeat_count=1
///   "ctrl+c+c"     -> modifiers={ctrl}, key=0x43, repeat_count=2
///   "shift+alt+r"  -> modifiers={shift,alt}, key=0x52, repeat_count=1
#[derive(Debug, Clone, PartialEq)]
struct ParsedHotkey {
    ctrl: bool,
    shift: bool,
    alt: bool,
    win: bool,
    /// The primary (non-modifier) virtual key code.
    vk_code: u32,
    /// How many times the key must be pressed in sequence (1 = single, 2 = double, etc.)
    repeat_count: u32,
}

impl ParsedHotkey {
    fn parse(hotkey_str: &str) -> Option<Self> {
        let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();
        if parts.is_empty() {
            return None;
        }

        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut win = false;
        let mut main_key: Option<u32> = None;
        let mut repeat_count: u32 = 0;

        for part in &parts {
            let lower = part.to_ascii_lowercase();
            match lower.as_str() {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" => alt = true,
                "win" | "super" | "meta" => win = true,
                _ => {
                    let vk = keys::key_name_to_vk(&lower)?.0 as u32;
                    if let Some(existing) = main_key {
                        if existing == vk {
                            // Same key repeated (e.g. ctrl+c+c)
                            repeat_count += 1;
                        } else {
                            // Different non-modifier keys — invalid
                            return None;
                        }
                    } else {
                        main_key = Some(vk);
                        repeat_count = 1;
                    }
                }
            }
        }

        let vk_code = main_key?;

        Some(ParsedHotkey {
            ctrl,
            shift,
            alt,
            win,
            vk_code,
            repeat_count,
        })
    }

    /// Check if the current modifiers match this hotkey's modifier requirements.
    fn modifiers_match(&self, modifiers: ModifierState) -> bool {
        self.ctrl == modifiers.ctrl
            && self.shift == modifiers.shift
            && self.alt == modifiers.alt
            && self.win == modifiers.win
    }

    /// Check if the given vk_code matches this hotkey's primary key.
    fn key_matches(&self, vk_code: u32) -> bool {
        self.vk_code == vk_code
    }

    /// Whether this hotkey requires multiple presses of the same key.
    fn is_multi_press(&self) -> bool {
        self.repeat_count > 1
    }
}

#[derive(Debug, Clone)]
struct RuntimeConfig {
    server_url: String,
    api_key: String,
    auto_detect_language: bool,
    target_language: String,
    hotkey_translate: ParsedHotkey,
    hotkey_reverse: ParsedHotkey,
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
            auto_detect_language: config.auto_detect_language,
            target_language: config.target_language.clone(),
            hotkey_translate: ParsedHotkey::parse(&config.hotkey_translate)
                .unwrap_or(ParsedHotkey {
                    ctrl: true,
                    shift: false,
                    alt: false,
                    win: false,
                    vk_code: 0x43, // C
                    repeat_count: 2,
                }),
            hotkey_reverse: ParsedHotkey::parse(&config.hotkey_reverse)
                .unwrap_or(ParsedHotkey {
                    ctrl: true,
                    shift: true,
                    alt: false,
                    win: false,
                    vk_code: 0x43, // C
                    repeat_count: 1,
                }),
        }
    }
}

#[derive(Debug, Default)]
struct MultiPressState {
    last_press: Option<Instant>,
    press_count: u32,
    clipboard_backup: Option<String>,
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
static TRANSLATE_PRESS_STATE: OnceLock<Mutex<MultiPressState>> = OnceLock::new();
static REVERSE_PRESS_STATE: OnceLock<Mutex<MultiPressState>> = OnceLock::new();

pub fn configure(config: &TranslateConfig) {
    let mut runtime_config = config_store()
        .lock()
        .expect("translate config mutex poisoned");
    *runtime_config = RuntimeConfig::from_config(config);
}

pub fn handle_keydown(vk_code: u32, modifiers: ModifierState, process_name: Option<&str>) -> bool {
    if exclusions::is_module_excluded(ModuleId::Translate, process_name) {
        return false;
    }

    let config = current_config();

    // Check translate hotkey
    let translate_result = check_hotkey(
        vk_code,
        modifiers,
        &config.hotkey_translate,
        translate_press_state(),
    );

    match translate_result {
        HotkeyMatchResult::Triggered => {
            trigger_translate(config);
            return true;
        }
        HotkeyMatchResult::Accumulating => {
            // For multi-press hotkeys like ctrl+c+c, don't consume the event
            // so Ctrl+C still performs copy on the first press.
            return false;
        }
        HotkeyMatchResult::NoMatch => {}
    }

    // Check reverse translate hotkey (only if not auto-detect)
    if !config.auto_detect_language {
        let reverse_result = check_hotkey(
            vk_code,
            modifiers,
            &config.hotkey_reverse,
            reverse_press_state(),
        );

        match reverse_result {
            HotkeyMatchResult::Triggered => {
                trigger_reverse_translate(config);
                return true;
            }
            HotkeyMatchResult::Accumulating => {
                return false;
            }
            HotkeyMatchResult::NoMatch => {}
        }
    }

    // Reset multi-press states if a non-matching key is pressed without the
    // required modifiers held.
    reset_state_if_needed(vk_code, modifiers, &config);

    false
}

#[derive(Debug, PartialEq)]
enum HotkeyMatchResult {
    /// The hotkey sequence is fully matched — action should fire.
    Triggered,
    /// A multi-press hotkey is accumulating presses (not yet complete).
    Accumulating,
    /// This keypress does not match the hotkey at all.
    NoMatch,
}

fn check_hotkey(
    vk_code: u32,
    modifiers: ModifierState,
    hotkey: &ParsedHotkey,
    state: &Mutex<MultiPressState>,
) -> HotkeyMatchResult {
    if !hotkey.modifiers_match(modifiers) || !hotkey.key_matches(vk_code) {
        return HotkeyMatchResult::NoMatch;
    }

    if !hotkey.is_multi_press() {
        // Single-press hotkey — trigger immediately
        return HotkeyMatchResult::Triggered;
    }

    // Multi-press hotkey (e.g. ctrl+c+c)
    let now = Instant::now();
    let mut press_state = state.lock().expect("press state mutex poisoned");

    let is_within_window = press_state
        .last_press
        .is_some_and(|last| now.duration_since(last) <= DOUBLE_C_WINDOW);

    if is_within_window {
        press_state.press_count += 1;
    } else {
        press_state.press_count = 1;
        press_state.clipboard_backup = crate::clipboard::current_text();
    }
    press_state.last_press = Some(now);

    if press_state.press_count >= hotkey.repeat_count {
        let backup = press_state.clipboard_backup.take();
        press_state.press_count = 0;
        press_state.last_press = None;
        drop(press_state);

        // For multi-press, we already have a copy in clipboard from the first press
        // Trigger translation from existing clipboard content
        trigger_translate_from_existing_copy(backup);
        return HotkeyMatchResult::Triggered;
    }

    HotkeyMatchResult::Accumulating
}

fn reset_state_if_needed(vk_code: u32, modifiers: ModifierState, config: &RuntimeConfig) {
    // If the translate hotkey is multi-press and the user presses something
    // that doesn't match while not holding the required modifiers, reset.
    if config.hotkey_translate.is_multi_press() {
        let should_reset = !config.hotkey_translate.modifiers_match(modifiers)
            || (!config.hotkey_translate.key_matches(vk_code)
                && !is_modifier_vk(vk_code));
        if should_reset {
            if let Ok(mut state) = translate_press_state().lock() {
                state.press_count = 0;
                state.last_press = None;
            }
        }
    }
    if config.hotkey_reverse.is_multi_press() {
        let should_reset = !config.hotkey_reverse.modifiers_match(modifiers)
            || (!config.hotkey_reverse.key_matches(vk_code)
                && !is_modifier_vk(vk_code));
        if should_reset {
            if let Ok(mut state) = reverse_press_state().lock() {
                state.press_count = 0;
                state.last_press = None;
            }
        }
    }
}

fn is_modifier_vk(vk_code: u32) -> bool {
    matches!(
        vk_code,
        0x10 | 0x11 | 0x12 |       // VK_SHIFT, VK_CONTROL, VK_MENU
        0xA0 | 0xA1 |               // VK_LSHIFT, VK_RSHIFT
        0xA2 | 0xA3 |               // VK_LCONTROL, VK_RCONTROL
        0xA4 | 0xA5 |               // VK_LMENU, VK_RMENU
        0x5B | 0x5C                  // VK_LWIN, VK_RWIN
    )
}

/// Trigger translate: for single-press hotkeys, we need to copy text first.
fn trigger_translate(config: RuntimeConfig) {
    if config.hotkey_translate.is_multi_press() {
        // Multi-press already handled via trigger_translate_from_existing_copy
        return;
    }
    // Single-press hotkey: copy selection, then translate
    let clipboard_backup = crate::clipboard::current_text();
    thread::spawn(move || {
        send_ctrl_c();
        thread::sleep(CLIPBOARD_SETTLE_DELAY);

        let Some(text) = crate::clipboard::current_text().filter(|text| !text.trim().is_empty()) else {
            crate::clipboard::restore_text(clipboard_backup);
            return;
        };

        crate::clipboard::restore_text(clipboard_backup);
        run_translation_flow(text, config, false);
    });
}

fn trigger_translate_from_existing_copy(clipboard_backup: Option<String>) {
    let config = current_config();
    thread::spawn(move || {
        thread::sleep(CLIPBOARD_SETTLE_DELAY);
        let Some(text) = crate::clipboard::current_text().filter(|text| !text.trim().is_empty()) else {
            crate::clipboard::restore_text(clipboard_backup);
            return;
        };

        crate::clipboard::restore_text(clipboard_backup);
        run_translation_flow(text, config, false);
    });
}

fn trigger_reverse_translate(config: RuntimeConfig) {
    let clipboard_backup = crate::clipboard::current_text();

    thread::spawn(move || {
        send_ctrl_c();
        thread::sleep(CLIPBOARD_SETTLE_DELAY);

        let Some(text) = crate::clipboard::current_text().filter(|text| !text.trim().is_empty()) else {
            crate::clipboard::restore_text(clipboard_backup);
            return;
        };

        crate::clipboard::restore_text(clipboard_backup);
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
    } else if config.auto_detect_language {
        auto_target_language(detected)
    } else {
        normalize_target_language(&config.target_language)
    };

    match translate_text(&text, &target, server_url, &config.api_key) {
        Ok(translated) => show_translation_toast(TranslationToastPayload {
            original: text,
            translated,
            source_lang: detected.to_string(),
            target_lang: target,
            reverse,
        }),
        Err(error) => show_translation_error(&format!("Ошибка перевода: {error}")),
    }
}

/// Lightweight language detection: returns "ru" if the text contains
/// any Cyrillic letter, otherwise "en". Good enough for a binary RU/EN choice.
fn detect_language(text: &str) -> &'static str {
    if text
        .chars()
        .any(|c| matches!(c, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}'))
    {
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

pub fn test_translate_api(server_url: &str, api_key: &str, target: &str) -> Result<String, String> {
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

    hide_translation_toast();

    thread::spawn(move || {
        let backup = crate::clipboard::current_text();

        if !crate::clipboard::set_text(&text) {
            return;
        }

        // Give the clipboard and the OS a moment to settle and refocus.
        thread::sleep(CLIPBOARD_SETTLE_DELAY);
        crate::clipboard::send_ctrl_v();

        // Wait for the target app to consume the paste before restoring the clipboard.
        thread::sleep(Duration::from_millis(200));
        crate::clipboard::restore_text(backup);
    });

    Ok(())
}

pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("нечего копировать".to_string());
    }

    Clipboard::new()
        .map_err(|error| format!("не удалось открыть буфер обмена ({error})"))?
        .set_text(text)
        .map_err(|error| format!("не удалось записать в буфер обмена ({error})"))
}

fn send_ctrl_c() {
    let modifiers = ModifierState::current();
    let held: [(bool, VIRTUAL_KEY); 3] = [
        (modifiers.shift, VK_SHIFT),
        (modifiers.alt, VK_MENU),
        (modifiers.win, VK_LWIN),
    ];

    let mut inputs: Vec<_> = Vec::new();

    for &(active, vkey) in &held {
        if active { inputs.push(press(vkey, true)); }
    }
    if !modifiers.ctrl {
        inputs.push(press(VK_CONTROL, false));
    }

    inputs.push(press_code(C_KEY, false));
    inputs.push(press_code(C_KEY, true));

    if !modifiers.ctrl {
        inputs.push(press(VK_CONTROL, true));
    }
    for &(active, vkey) in &held {
        if active { inputs.push(press(vkey, false)); }
    }

    keys::send_inputs(&inputs);
}

fn reverse_target_language(target_language: &str) -> String {
    if target_language.eq_ignore_ascii_case("ru") {
        "en".to_string()
    } else {
        "ru".to_string()
    }
}

fn auto_target_language(detected: &str) -> String {
    match detected {
        "ru" => "en".to_string(),
        _ => "ru".to_string(),
    }
}

fn normalize_target_language(target_language: &str) -> String {
    if target_language.eq_ignore_ascii_case("en") {
        "en".to_string()
    } else {
        "ru".to_string()
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

fn translate_press_state() -> &'static Mutex<MultiPressState> {
    TRANSLATE_PRESS_STATE.get_or_init(|| Mutex::new(MultiPressState::default()))
}

fn reverse_press_state() -> &'static Mutex<MultiPressState> {
    REVERSE_PRESS_STATE.get_or_init(|| Mutex::new(MultiPressState::default()))
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

    #[test]
    fn libretranslate_request_omits_empty_api_key() {
        let request = LibreTranslateRequest {
            q: "hello",
            source: "auto",
            target: "ru",
            format: "text",
            api_key: None,
        };

        let json = serde_json::to_value(request).expect("serialize request");

        assert_eq!(json["q"], "hello");
        assert_eq!(json["source"], "auto");
        assert_eq!(json["target"], "ru");
        assert!(json.get("api_key").is_none());
    }

    #[test]
    fn test_translate_api_validates_local_inputs_before_network() {
        assert_eq!(
            test_translate_api("", "", "ru"),
            Err("Адрес сервера LibreTranslate не настроен".to_string())
        );
        assert_eq!(
            test_translate_api("http://127.0.0.1:5000", "", "de"),
            Err("целевой язык должен быть 'ru' или 'en'".to_string())
        );
    }

    #[test]
    fn parsed_hotkey_simple_combo() {
        let hk = ParsedHotkey::parse("ctrl+alt+r").unwrap();
        assert!(hk.ctrl);
        assert!(!hk.shift);
        assert!(hk.alt);
        assert!(!hk.win);
        assert_eq!(hk.vk_code, 0x52); // 'R'
        assert_eq!(hk.repeat_count, 1);
        assert!(!hk.is_multi_press());
    }

    #[test]
    fn parsed_hotkey_shift_alt() {
        let hk = ParsedHotkey::parse("shift+alt+r").unwrap();
        assert!(!hk.ctrl);
        assert!(hk.shift);
        assert!(hk.alt);
        assert!(!hk.win);
        assert_eq!(hk.vk_code, 0x52); // 'R'
        assert_eq!(hk.repeat_count, 1);
    }

    #[test]
    fn parsed_hotkey_multi_press() {
        let hk = ParsedHotkey::parse("ctrl+c+c").unwrap();
        assert!(hk.ctrl);
        assert!(!hk.shift);
        assert!(!hk.alt);
        assert!(!hk.win);
        assert_eq!(hk.vk_code, 0x43); // 'C'
        assert_eq!(hk.repeat_count, 2);
        assert!(hk.is_multi_press());
    }

    #[test]
    fn parsed_hotkey_ctrl_shift_c() {
        let hk = ParsedHotkey::parse("ctrl+shift+c").unwrap();
        assert!(hk.ctrl);
        assert!(hk.shift);
        assert!(!hk.alt);
        assert!(!hk.win);
        assert_eq!(hk.vk_code, 0x43); // 'C'
        assert_eq!(hk.repeat_count, 1);
    }

    #[test]
    fn parsed_hotkey_function_key() {
        let hk = ParsedHotkey::parse("ctrl+f5").unwrap();
        assert!(hk.ctrl);
        assert_eq!(hk.vk_code, 0x74); // VK_F5 = 0x6F + 5
        assert_eq!(hk.repeat_count, 1);
    }

    #[test]
    fn parsed_hotkey_modifiers_match() {
        let hk = ParsedHotkey::parse("ctrl+alt+r").unwrap();
        let modifiers = ModifierState {
            ctrl: true,
            shift: false,
            alt: true,
            win: false,
        };
        assert!(hk.modifiers_match(modifiers));
        assert!(hk.key_matches(0x52));

        let wrong_modifiers = ModifierState {
            ctrl: true,
            shift: true,
            alt: true,
            win: false,
        };
        assert!(!hk.modifiers_match(wrong_modifiers));
    }

    #[test]
    fn parsed_hotkey_invalid_input() {
        assert!(ParsedHotkey::parse("").is_none());
        assert!(ParsedHotkey::parse("ctrl+a+b").is_none()); // two different non-modifier keys
    }
}
