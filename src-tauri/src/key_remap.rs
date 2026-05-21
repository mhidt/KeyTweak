use crate::config::KeyRemapConfig;
use std::{
    mem::size_of,
    sync::{Mutex, OnceLock},
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT,
    VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
};

#[derive(Debug, Clone, Default)]
struct RuntimeConfig {
    enabled: bool,
    mappings: Vec<KeyMapping>,
}

#[derive(Debug, Clone)]
struct KeyMapping {
    from: RemapKey,
    to: RemapKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemapKey {
    Alt,
    Backspace,
    CapsLock,
    Ctrl,
    Enter,
    Esc,
    Shift,
    Space,
    Tab,
    Win,
}

static CONFIG: OnceLock<Mutex<RuntimeConfig>> = OnceLock::new();

pub fn configure(config: &KeyRemapConfig) {
    let mappings = config
        .mappings
        .iter()
        .filter(|mapping| mapping.enabled)
        .filter_map(|mapping| {
            let from = RemapKey::from_id(&mapping.from)?;
            let to = RemapKey::from_id(&mapping.to)?;
            (from != to).then_some(KeyMapping { from, to })
        })
        .collect();

    let mut runtime_config = config_store()
        .lock()
        .expect("key remap config mutex poisoned");
    *runtime_config = RuntimeConfig {
        enabled: config.enabled,
        mappings,
    };
}

pub fn handle_key_event(vk_code: u32, is_keyup: bool) -> bool {
    let config = config_store()
        .lock()
        .expect("key remap config mutex poisoned")
        .clone();

    if !config.enabled {
        return false;
    }

    let Some(mapping) = config
        .mappings
        .iter()
        .find(|mapping| mapping.from.matches_vk(vk_code))
    else {
        return false;
    };

    send_key(mapping.to.output_vk(), is_keyup);
    true
}

impl RemapKey {
    fn from_id(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "alt" => Some(Self::Alt),
            "backspace" => Some(Self::Backspace),
            "caps_lock" => Some(Self::CapsLock),
            "ctrl" => Some(Self::Ctrl),
            "enter" => Some(Self::Enter),
            "esc" => Some(Self::Esc),
            "shift" => Some(Self::Shift),
            "space" => Some(Self::Space),
            "tab" => Some(Self::Tab),
            "win" => Some(Self::Win),
            _ => None,
        }
    }

    fn matches_vk(self, vk_code: u32) -> bool {
        match self {
            Self::Alt => matches_vk(vk_code, &[VK_MENU, VK_LMENU, VK_RMENU]),
            Self::Backspace => matches_vk(vk_code, &[VK_BACK]),
            Self::CapsLock => matches_vk(vk_code, &[VK_CAPITAL]),
            Self::Ctrl => matches_vk(vk_code, &[VK_CONTROL, VK_LCONTROL, VK_RCONTROL]),
            Self::Enter => vk_code == 0x0D,
            Self::Esc => matches_vk(vk_code, &[VK_ESCAPE]),
            Self::Shift => matches_vk(vk_code, &[VK_SHIFT, VK_LSHIFT, VK_RSHIFT]),
            Self::Space => matches_vk(vk_code, &[VK_SPACE]),
            Self::Tab => matches_vk(vk_code, &[VK_TAB]),
            Self::Win => matches_vk(vk_code, &[VK_LWIN, VK_RWIN]),
        }
    }

    fn output_vk(self) -> VIRTUAL_KEY {
        match self {
            Self::Alt => VK_LMENU,
            Self::Backspace => VK_BACK,
            Self::CapsLock => VK_CAPITAL,
            Self::Ctrl => VK_LCONTROL,
            Self::Enter => VIRTUAL_KEY(0x0D),
            Self::Esc => VK_ESCAPE,
            Self::Shift => VK_LSHIFT,
            Self::Space => VK_SPACE,
            Self::Tab => VK_TAB,
            Self::Win => VK_LWIN,
        }
    }
}

fn matches_vk(vk_code: u32, keys: &[VIRTUAL_KEY]) -> bool {
    keys.iter().any(|key| vk_code == key.0 as u32)
}

fn send_key(vk: VIRTUAL_KEY, is_keyup: bool) {
    let mut flags = if is_keyup {
        KEYEVENTF_KEYUP
    } else {
        Default::default()
    };
    if is_extended_key(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        let _ = SendInput(&[input], size_of::<INPUT>() as i32);
    }
}

fn is_extended_key(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_LWIN | VK_RWIN | VK_LMENU | VK_RMENU | VK_LCONTROL | VK_RCONTROL
    )
}

fn config_store() -> &'static Mutex<RuntimeConfig> {
    CONFIG.get_or_init(|| Mutex::new(RuntimeConfig::default()))
}
