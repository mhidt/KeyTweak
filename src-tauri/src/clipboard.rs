use crate::keys::{self, press, press_code};
use arboard::Clipboard;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;

const V_KEY: u32 = 0x56;

pub fn current_text() -> Option<String> {
    Clipboard::new().ok()?.get_text().ok()
}

pub fn set_text(text: &str) -> bool {
    match Clipboard::new() {
        Ok(mut clipboard) => clipboard.set_text(text).is_ok(),
        Err(_) => false,
    }
}

pub fn restore_text(text: Option<String>) {
    if let Some(text) = text {
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }
}

pub fn send_ctrl_v() {
    let inputs = [
        press(VK_CONTROL, false),
        press_code(V_KEY, false),
        press_code(V_KEY, true),
        press(VK_CONTROL, true),
    ];

    keys::send_inputs(&inputs);
}
