use std::mem::size_of;
use windows::Win32::{
    UI::{
        Input::KeyboardAndMouse::{
            GetKeyboardLayout, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
            KEYBDINPUT, KEYBD_EVENT_FLAGS, INPUT, INPUT_0, INPUT_KEYBOARD, SendInput, VIRTUAL_KEY,
            VK_BACK, VK_CAPITAL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT,
            VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_NEXT, VK_PRIOR, VK_RCONTROL,
            VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SPACE, VK_TAB, VK_UP,
        },
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

const EXTENDED_KEYS: &[(&str, VIRTUAL_KEY)] = &[
    ("end", VK_END),
    ("home", VK_HOME),
    ("insert", VK_INSERT),
    ("delete", VK_DELETE),
    ("pageup", VK_PRIOR),
    ("pagedown", VK_NEXT),
    ("arrowleft", VK_LEFT),
    ("arrowright", VK_RIGHT),
    ("arrowup", VK_UP),
    ("arrowdown", VK_DOWN),
    ("left_alt", VK_LMENU),
    ("right_alt", VK_RMENU),
    ("left_control", VK_LCONTROL),
    ("right_control", VK_RCONTROL),
    ("left_win", VK_LWIN),
    ("right_win", VK_RWIN),
    ("left_shift", VK_LSHIFT),
    ("right_shift", VK_RSHIFT),
];

const PREFIX_KEYS: &[(&str, u16, u16, u16)] = &[
    ("f", 1, 24, 0x6F),
    ("num", 0, 9, 0x60),
];

/// Maps a browser-native key name to a VIRTUAL_KEY code.
pub fn key_name_to_vk(name: &str) -> Option<VIRTUAL_KEY> {
    if let Some(&(_, vk)) = EXTENDED_KEYS.iter().find(|(n, _)| n == &name) {
        return Some(vk);
    }

    let vk: VIRTUAL_KEY = match name {
        "backspace" => VK_BACK,
        "tab" => VK_TAB,
        "enter" => VIRTUAL_KEY(0x0D),
        "escape" => VK_ESCAPE,
        "space" => VK_SPACE,
        "capslock" => VK_CAPITAL,
        _ => {
            if name.len() == 1 {
                let byte = name.as_bytes()[0];
                if byte.is_ascii_alphanumeric() {
                    return Some(VIRTUAL_KEY(byte.to_ascii_uppercase() as u16));
                }
                return None;
            }
            for &(prefix, min, max, base) in PREFIX_KEYS {
                if let Some(num_str) = name.strip_prefix(prefix) {
                    if let Ok(n) = num_str.parse::<u16>() {
                        if (min..=max).contains(&n) {
                            return Some(VIRTUAL_KEY(base + n));
                        }
                    }
                    return None;
                }
            }
            return None;
        }
    };
    Some(vk)
}

/// Creates a keyboard INPUT event for a virtual key press/release.
pub fn press(key: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let mut flags = if key_up { KEYEVENTF_KEYUP } else { Default::default() };
    if is_extended_key(key) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    keyboard_input(key, 0, flags)
}

/// Shorthand for `press(VIRTUAL_KEY(code), key_up)`.
pub fn press_code(code: u32, key_up: bool) -> INPUT {
    press(VIRTUAL_KEY(code as u16), key_up)
}

/// Creates a keyboard INPUT event for a Unicode character press/release.
pub fn unicode(unit: u16, key_up: bool) -> INPUT {
    let flags = KEYEVENTF_UNICODE | if key_up { KEYEVENTF_KEYUP } else { Default::default() };
    keyboard_input(VIRTUAL_KEY(0), unit, flags)
}

fn keyboard_input(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Sends a slice of INPUT events and returns the number actually sent.
pub fn send_inputs(inputs: &[INPUT]) -> u32 {
    unsafe { SendInput(inputs, size_of::<INPUT>() as i32) }
}

/// Returns the keyboard layout (HKL) for the foreground window's thread.
pub fn foreground_layout() -> windows::Win32::UI::Input::KeyboardAndMouse::HKL {
    let foreground = unsafe { GetForegroundWindow() };
    let thread_id = if foreground.is_invalid() {
        0
    } else {
        unsafe { GetWindowThreadProcessId(foreground, None) }
    };
    unsafe { GetKeyboardLayout(thread_id) }
}

fn is_extended_key(vk: VIRTUAL_KEY) -> bool {
    EXTENDED_KEYS.iter().any(|&(_, ext_vk)| vk == ext_vk)
}
