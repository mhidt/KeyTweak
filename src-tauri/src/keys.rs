use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_BACK, VK_CAPITAL,
    VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU,
    VK_LSHIFT, VK_LWIN, VK_NEXT, VK_PRIOR, VK_RCONTROL, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN,
    VK_SPACE, VK_TAB, VK_UP,
};

/// Maps a browser-native key name to a VIRTUAL_KEY code.
pub fn key_name_to_vk(name: &str) -> Option<VIRTUAL_KEY> {
    let vk: VIRTUAL_KEY = match name {
        "backspace" => VK_BACK,
        "tab" => VK_TAB,
        "enter" => VIRTUAL_KEY(0x0D),
        "escape" => VK_ESCAPE,
        "space" => VK_SPACE,
        "pageup" => VK_PRIOR,
        "pagedown" => VK_NEXT,
        "end" => VK_END,
        "home" => VK_HOME,
        "arrowleft" => VK_LEFT,
        "arrowup" => VK_UP,
        "arrowright" => VK_RIGHT,
        "arrowdown" => VK_DOWN,
        "insert" => VK_INSERT,
        "delete" => VK_DELETE,
        "capslock" => VK_CAPITAL,
        "left_alt" => VK_LMENU,
        "left_control" => VK_LCONTROL,
        "left_shift" => VK_LSHIFT,
        "left_win" => VK_LWIN,
        "right_alt" => VK_RMENU,
        "right_control" => VK_RCONTROL,
        "right_shift" => VK_RSHIFT,
        "right_win" => VK_RWIN,
        _ => {
            if name.len() == 1 {
                let byte = name.as_bytes()[0];
                if byte.is_ascii_alphabetic() {
                    return Some(VIRTUAL_KEY(byte.to_ascii_uppercase() as u16));
                } else if byte.is_ascii_digit() {
                    return Some(VIRTUAL_KEY(byte as u16));
                }
                return None;
            }
            if let Some(num_str) = name.strip_prefix('f') {
                let n = num_str.parse::<u16>().ok()?;
                if (1..=24).contains(&n) {
                    return Some(VIRTUAL_KEY(0x6F + n));
                }
                return None;
            }
            if let Some(num_str) = name.strip_prefix("numpad") {
                let n = num_str.parse::<u16>().ok()?;
                if n <= 9 {
                    return Some(VIRTUAL_KEY(0x60 + n));
                }
                return None;
            }
            return None;
        }
    };
    Some(vk)
}

/// Creates a keyboard INPUT event for a virtual key press/release.
pub fn vk(key: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
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
