use crate::{
    config::{AutoReplaceConfig, ExceptionMode, ProgramException, Replacement},
    keyboard_hook::ModifierState,
    keys::{self, press, unicode},
};
use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::OsStringExt,
    path::Path,
    sync::{Mutex, OnceLock},
};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetKeyboardState, ToUnicodeEx, VK_BACK, VK_DELETE, VK_DOWN, VK_END,
                VK_ESCAPE, VK_HOME, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
            },
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    },
};

const MAX_BUFFER_CHARS: usize = 128;
const PUNCTUATION_TRIGGERS: &[char] = &['.', ',', '?', '!', ';', ':'];

#[derive(Debug, Clone)]
struct RuntimeConfig {
    trigger_space: bool,
    trigger_tab: bool,
    trigger_enter: bool,
    trigger_punctuation: bool,
    whole_words_only: bool,
    case_sensitive: bool,
    replacements: Vec<Replacement>,
    exceptions: Vec<ProgramException>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::from_config(&AutoReplaceConfig::default())
    }
}

impl RuntimeConfig {
    fn from_config(config: &AutoReplaceConfig) -> Self {
        Self {
            trigger_space: config.trigger_space,
            trigger_tab: config.trigger_tab,
            trigger_enter: config.trigger_enter,
            trigger_punctuation: config.trigger_punctuation,
            whole_words_only: config.whole_words_only,
            case_sensitive: config.case_sensitive,
            replacements: config.replacements.clone(),
            exceptions: config.exceptions.clone(),
        }
    }
}

#[derive(Debug, Default)]
struct InputBuffer {
    foreground: isize,
    word: String,
}

static CONFIG: OnceLock<Mutex<RuntimeConfig>> = OnceLock::new();
static BUFFER: OnceLock<Mutex<InputBuffer>> = OnceLock::new();

pub fn configure(config: &AutoReplaceConfig) {
    let mut runtime_config = config_store()
        .lock()
        .expect("autoreplace config mutex poisoned");
    *runtime_config = RuntimeConfig::from_config(config);
}

pub fn handle_keydown(vk_code: u32, scan_code: u32, modifiers: ModifierState) -> bool {
    if modifiers.ctrl || modifiers.alt || modifiers.win {
        clear_buffer();
        return false;
    }

    let config = config_store()
        .lock()
        .expect("autoreplace config mutex poisoned")
        .clone();

    if is_excluded(&config) {
        clear_buffer();
        return false;
    }

    sync_foreground_buffer();

    if let Some(separator) = separator_for_vk(vk_code, &config) {
        return handle_separator(separator, &config);
    }

    if vk_code == VK_BACK.0 as u32 {
        let mut buffer = buffer_store()
            .lock()
            .expect("autoreplace buffer mutex poisoned");
        buffer.word.pop();
        return false;
    }

    if should_clear_buffer(vk_code) {
        clear_buffer();
        return false;
    }

    if let Some(ch) = key_to_char(vk_code, scan_code) {
        let mut buffer = buffer_store()
            .lock()
            .expect("autoreplace buffer mutex poisoned");
        buffer.word.push(ch);

        if buffer.word.chars().count() > MAX_BUFFER_CHARS {
            buffer.word.clear();
        }
    }

    false
}

fn handle_separator(separator: char, config: &RuntimeConfig) -> bool {
    let word = {
        let buffer = buffer_store()
            .lock()
            .expect("autoreplace buffer mutex poisoned");
        buffer.word.clone()
    };

    if word.is_empty() {
        // No word collected yet, but a punctuation trigger may itself start
        // a new shortcut (e.g. ":bug"). Whitespace separators don't belong
        // inside a shortcut and are skipped.
        if PUNCTUATION_TRIGGERS.contains(&separator) {
            push_to_buffer(separator);
        }
        return false;
    }

    if let Some(replacement) = find_replacement(config, &word) {
        if !replace_word(&word, &replacement) {
            log::error!("failed to send autoreplace input for separator '{separator}'");
        }
        clear_buffer();
    } else {
        clear_buffer();
        // No replacement matched: if the separator is punctuation it might
        // be the first char of a new shortcut, so seed the buffer with it.
        if PUNCTUATION_TRIGGERS.contains(&separator) {
            push_to_buffer(separator);
        }
    }

    // Always let the separator key pass through to the OS so the trigger
    // character (space, tab, punctuation) is preserved in the output.
    false
}

fn push_to_buffer(ch: char) {
    let mut buffer = buffer_store()
        .lock()
        .expect("autoreplace buffer mutex poisoned");
    buffer.word.push(ch);
    if buffer.word.chars().count() > MAX_BUFFER_CHARS {
        buffer.word.clear();
    }
}

fn find_replacement(config: &RuntimeConfig, word: &str) -> Option<String> {
    if !config.whole_words_only && word.is_empty() {
        return None;
    }

    let needle = if config.case_sensitive {
        word.to_string()
    } else {
        word.to_lowercase()
    };

    config.replacements.iter().find_map(|entry| {
        let short = if config.case_sensitive {
            entry.short.clone()
        } else {
            entry.short.to_lowercase()
        };

        let matched = if config.whole_words_only {
            needle == short
        } else {
            needle.ends_with(&short)
        };

        matched.then(|| entry.replacement.clone())
    })
}

fn separator_for_vk(vk_code: u32, config: &RuntimeConfig) -> Option<char> {
    match vk_code {
        code if code == VK_SPACE.0 as u32 && config.trigger_space => Some(' '),
        code if code == VK_TAB.0 as u32 && config.trigger_tab => Some('\t'),
        code if code == VK_RETURN.0 as u32 && config.trigger_enter => Some('\n'),
        _ => key_to_char(vk_code, 0)
            .filter(|ch| config.trigger_punctuation && PUNCTUATION_TRIGGERS.contains(ch)),
    }
}

fn key_to_char(vk_code: u32, scan_code: u32) -> Option<char> {
    let mut keyboard_state = [0u8; 256];
    unsafe { GetKeyboardState(&mut keyboard_state) }.ok()?;

    let layout = keys::foreground_layout();
    let mut chars = [0u16; 8];
    let count = unsafe { ToUnicodeEx(vk_code, scan_code, &keyboard_state, &mut chars, 0, layout) };

    if count <= 0 {
        return None;
    }

    char::decode_utf16(chars.into_iter().take(count as usize))
        .next()
        .and_then(Result::ok)
        .filter(|ch| !ch.is_control())
}

fn replace_word(word: &str, replacement: &str) -> bool {
    let mut inputs = Vec::new();

    for _ in word.chars() {
        inputs.push(press(VK_BACK, false));
        inputs.push(press(VK_BACK, true));
    }

    for unit in replacement.encode_utf16() {
        inputs.push(unicode(unit, false));
        inputs.push(unicode(unit, true));
    }

    keys::send_inputs(&inputs) == inputs.len() as u32
}

fn should_clear_buffer(vk_code: u32) -> bool {
    [VK_ESCAPE, VK_DELETE, VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN, VK_HOME, VK_END]
        .iter()
        .any(|vk| vk.0 as u32 == vk_code)
}

fn is_excluded(config: &RuntimeConfig) -> bool {
    let Some(process_name) = foreground_process_name() else {
        return false;
    };

    let (in_blacklist, in_whitelist, has_whitelist) = config.exceptions.iter().fold(
        (false, false, false),
        |(in_bl, in_wl, has_wl), entry| {
            let matches = same_process_name(&entry.program, &process_name);
            (
                in_bl || (entry.mode == ExceptionMode::Blacklist && matches),
                in_wl || (entry.mode == ExceptionMode::Whitelist && matches),
                has_wl || entry.mode == ExceptionMode::Whitelist,
            )
        },
    );

    in_blacklist || (has_whitelist && !in_whitelist)
}

fn foreground_process_name() -> Option<String> {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_invalid() {
        return None;
    }

    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(foreground, Some(&mut process_id)) };
    if process_id == 0 {
        return None;
    }

    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let mut buffer = [0u16; 32768];
    let mut size = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }

    result.ok()?;

    let path = OsString::from_wide(&buffer[..size as usize]);
    filename_lower(&path)
}

fn filename_lower(path: &OsStr) -> Option<String> {
    Path::new(path).file_name().map(|n| n.to_string_lossy().to_lowercase())
}

fn same_process_name(configured: &str, actual: &str) -> bool {
    let configured = filename_lower(OsStr::new(configured)).unwrap_or_else(|| configured.to_lowercase());

    configured == actual
}

fn sync_foreground_buffer() {
    let foreground = unsafe { GetForegroundWindow() }.0 as isize;
    let mut buffer = buffer_store()
        .lock()
        .expect("autoreplace buffer mutex poisoned");

    if buffer.foreground != foreground {
        buffer.foreground = foreground;
        buffer.word.clear();
    }
}

fn clear_buffer() {
    if let Ok(mut buffer) = buffer_store().lock() {
        buffer.word.clear();
    }
}

fn config_store() -> &'static Mutex<RuntimeConfig> {
    CONFIG.get_or_init(|| Mutex::new(RuntimeConfig::default()))
}

fn buffer_store() -> &'static Mutex<InputBuffer> {
    BUFFER.get_or_init(|| Mutex::new(InputBuffer::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_replacements() -> RuntimeConfig {
        let mut config = RuntimeConfig::default();
        config.replacements = vec![Replacement {
            short: "почта".to_string(),
            replacement: "myemail@gmail.com".to_string(),
        }];
        config
    }

    #[test]
    fn matches_case_insensitive_whole_word() {
        let config = config_with_replacements();

        assert_eq!(
            find_replacement(&config, "ПОЧТА"),
            Some("myemail@gmail.com".to_string())
        );
    }

    #[test]
    fn respects_case_sensitive_option() {
        let mut config = config_with_replacements();
        config.case_sensitive = true;

        assert_eq!(find_replacement(&config, "ПОЧТА"), None);
        assert_eq!(
            find_replacement(&config, "почта"),
            Some("myemail@gmail.com".to_string())
        );
    }

    #[test]
    fn supports_suffix_match_when_whole_words_disabled() {
        let mut config = config_with_replacements();
        config.whole_words_only = false;

        assert_eq!(
            find_replacement(&config, "мояпочта"),
            Some("myemail@gmail.com".to_string())
        );
    }

    #[test]
    fn separator_toggles_work() {
        let mut config = RuntimeConfig::default();
        config.trigger_space = true;
        config.trigger_tab = false;
        config.trigger_enter = false;

        assert_eq!(separator_for_vk(VK_SPACE.0 as u32, &config), Some(' '));
        assert_eq!(separator_for_vk(VK_TAB.0 as u32, &config), None);
        assert_eq!(separator_for_vk(VK_RETURN.0 as u32, &config), None);
    }

    #[test]
    fn process_name_matching_uses_file_name() {
        assert!(same_process_name("code.exe", "code.exe"));
        assert!(same_process_name(
            r"C:\Program Files\App\code.exe",
            "code.exe"
        ));
        assert!(!same_process_name("notepad.exe", "code.exe"));
    }
}
