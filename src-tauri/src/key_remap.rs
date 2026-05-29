use crate::{config::KeyRemapConfig, keys::{self, press}};
use std::{
    sync::{Mutex, OnceLock},
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
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
    AnyAlt,
    AnyCtrl,
    AnyShift,
    AnyWin,
    Vk(VIRTUAL_KEY),
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

/// Returns true if there is an active mapping whose source matches Right Alt
/// (either `VK_RMENU` specifically or the `AnyAlt` wildcard).
pub fn has_right_alt_mapping() -> bool {
    let config = config_store()
        .lock()
        .expect("key remap config mutex poisoned");

    if !config.enabled {
        return false;
    }

    config
        .mappings
        .iter()
        .any(|m| m.from.matches_vk(VK_RMENU.0 as u32))
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

    send_key(mapping.to.output_vk(vk_code), is_keyup);
    true
}

impl RemapKey {
    fn from_id(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "alt" => Some(Self::AnyAlt),
            "ctrl" | "control" => Some(Self::AnyCtrl),
            "shift" => Some(Self::AnyShift),
            "win" => Some(Self::AnyWin),
            name => keys::key_name_to_vk(name).map(Self::Vk),
        }
    }

    fn matches_vk(self, vk_code: u32) -> bool {
        match self {
            Self::AnyAlt => matches_vk(vk_code, &[VK_MENU, VK_LMENU, VK_RMENU]),
            Self::AnyCtrl => matches_vk(vk_code, &[VK_CONTROL, VK_LCONTROL, VK_RCONTROL]),
            Self::AnyShift => matches_vk(vk_code, &[VK_SHIFT, VK_LSHIFT, VK_RSHIFT]),
            Self::AnyWin => matches_vk(vk_code, &[VK_LWIN, VK_RWIN]),
            Self::Vk(vk) => match vk {
                VK_LCONTROL => vk_code == VK_LCONTROL.0 as u32,
                VK_RCONTROL => vk_code == VK_RCONTROL.0 as u32,
                VK_LMENU => vk_code == VK_LMENU.0 as u32,
                VK_RMENU => vk_code == VK_RMENU.0 as u32,
                VK_LSHIFT => vk_code == VK_LSHIFT.0 as u32,
                VK_RSHIFT => vk_code == VK_RSHIFT.0 as u32,
                VK_LWIN => vk_code == VK_LWIN.0 as u32,
                VK_RWIN => vk_code == VK_RWIN.0 as u32,
                _ => vk_code == vk.0 as u32,
            },
        }
    }

    fn output_vk(self, source_vk_code: u32) -> VIRTUAL_KEY {
        match self {
            Self::AnyAlt => side_matching_vk(source_vk_code, VK_LMENU, VK_RMENU),
            Self::AnyCtrl => side_matching_vk(source_vk_code, VK_LCONTROL, VK_RCONTROL),
            Self::AnyShift => side_matching_vk(source_vk_code, VK_LSHIFT, VK_RSHIFT),
            Self::AnyWin => side_matching_vk(source_vk_code, VK_LWIN, VK_RWIN),
            Self::Vk(vk) => vk,
        }
    }
}

fn side_matching_vk(source_vk_code: u32, left: VIRTUAL_KEY, right: VIRTUAL_KEY) -> VIRTUAL_KEY {
    if source_vk_code == right.0 as u32 {
        right
    } else {
        left
    }
}

fn matches_vk(vk_code: u32, keys: &[VIRTUAL_KEY]) -> bool {
    keys.iter().any(|key| vk_code == key.0 as u32)
}

fn send_key(vk: VIRTUAL_KEY, is_keyup: bool) {
    let input = press(vk, is_keyup);
    keys::send_inputs(&[input]);
}

fn config_store() -> &'static Mutex<RuntimeConfig> {
    CONFIG.get_or_init(|| Mutex::new(RuntimeConfig::default()))
}
