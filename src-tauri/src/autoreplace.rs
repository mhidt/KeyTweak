use crate::{
    config::{AutoReplaceConfig, Replacement},
    config::ModuleId,
    exclusions,
    keyboard_hook::ModifierState,
    keys::{self, press, unicode},
};
use std::sync::{Mutex, OnceLock};
use windows::Win32::UI::{
    Input::KeyboardAndMouse::{
        GetKeyboardState, ToUnicodeEx, VK_BACK, VK_DELETE, VK_DOWN, VK_END,
        VK_ESCAPE, VK_HOME, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
    },
    WindowsAndMessaging::GetForegroundWindow,
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

pub fn handle_keydown(vk_code: u32, scan_code: u32, modifiers: ModifierState, process_name: Option<&str>) -> bool {
    if modifiers.ctrl || modifiers.alt || modifiers.win {
        clear_buffer();
        return false;
    }

    let config = config_store()
        .lock()
        .expect("autoreplace config mutex poisoned")
        .clone();

    if exclusions::is_module_excluded(ModuleId::AutoReplace, process_name) {
        clear_buffer();
        return false;
    }

    sync_foreground_buffer();

    if let Some(separator) = separator_for_vk(vk_code, &config) {
        return handle_separator(separator, &config, process_name);
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

fn handle_separator(separator: char, config: &RuntimeConfig, process_name: Option<&str>) -> bool {
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

    if let Some(replacement) = find_replacement(config, &word, process_name) {
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

fn find_replacement(config: &RuntimeConfig, word: &str, process_name: Option<&str>) -> Option<String> {
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

        if !matched || is_replacement_excluded(entry, process_name) {
            return None;
        }

        Some(entry.replacement.clone())
    })
}

/// Returns `true` if this replacement is blacklisted for the current foreground
/// program (per-replacement exclusion list).
fn is_replacement_excluded(entry: &Replacement, process_name: Option<&str>) -> bool {
    if entry.exclusions.is_empty() {
        return false;
    }

    let Some(process_name) = process_name else {
        return false;
    };

    entry
        .exclusions
        .iter()
        .any(|excluded| exclusions::program_matches(&excluded.program, process_name))
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
            exclusions: Vec::new(),
        }];
        config
    }

    #[test]
    fn matches_case_insensitive_whole_word() {
        let config = config_with_replacements();

        assert_eq!(
            find_replacement(&config, "ПОЧТА", None),
            Some("myemail@gmail.com".to_string())
        );
    }

    #[test]
    fn respects_case_sensitive_option() {
        let mut config = config_with_replacements();
        config.case_sensitive = true;

        assert_eq!(find_replacement(&config, "ПОЧТА", None), None);
        assert_eq!(
            find_replacement(&config, "почта", None),
            Some("myemail@gmail.com".to_string())
        );
    }

    #[test]
    fn supports_suffix_match_when_whole_words_disabled() {
        let mut config = config_with_replacements();
        config.whole_words_only = false;

        assert_eq!(
            find_replacement(&config, "мояпочта", None),
            Some("myemail@gmail.com".to_string())
        );
    }

    #[test]
    fn skips_replacement_excluded_for_current_program() {
        use crate::config::ExceptionProgram;

        let mut config = config_with_replacements();
        config.replacements[0].exclusions = vec![ExceptionProgram {
            program: "code.exe".to_string(),
            display_name: None,
        }];

        // Excluded program: no replacement.
        assert_eq!(find_replacement(&config, "почта", Some("code.exe")), None);
        // Excluded program given as full path still matches by file name.
        assert_eq!(
            find_replacement(&config, "почта", Some("code.exe")),
            find_replacement(&config, "почта", Some("code.exe"))
        );
        // Different program: replacement still works.
        assert_eq!(
            find_replacement(&config, "почта", Some("notepad.exe")),
            Some("myemail@gmail.com".to_string())
        );
        // Unknown foreground program: replacement works.
        assert_eq!(
            find_replacement(&config, "почта", None),
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
}
