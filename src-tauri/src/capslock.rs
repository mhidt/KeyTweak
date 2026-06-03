use crate::{
    config::{CapsLockConfig, ModuleId, RealCapsCombo},
    exclusions,
    keyboard_hook::ModifierState,
    keys,
};
use std::sync::{Mutex, OnceLock};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CAPITAL, VK_LWIN, VK_SPACE};

#[derive(Debug, Clone, Copy)]
struct RuntimeSettings {
    real_caps_combo: RealCapsCombo,
    /// Virtual-key code of the configured trigger key. `None` if the configured
    /// key name could not be resolved (the language switch is then disabled).
    switch_key_vk: Option<u32>,
    paused: bool,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        let config = CapsLockConfig::default();

        Self {
            real_caps_combo: config.real_caps_combo,
            switch_key_vk: resolve_switch_key(&config.switch_key),
            paused: config.paused,
        }
    }
}

fn resolve_switch_key(name: &str) -> Option<u32> {
    keys::key_name_to_vk(name).map(|vk| vk.0 as u32)
}

static SETTINGS: OnceLock<Mutex<RuntimeSettings>> = OnceLock::new();

pub fn configure(config: &CapsLockConfig) {
    let mut settings = settings().lock().expect("capslock settings mutex poisoned");
    *settings = RuntimeSettings {
        real_caps_combo: config.real_caps_combo,
        switch_key_vk: resolve_switch_key(&config.switch_key),
        paused: config.paused,
    };
}

/// Returns the virtual-key code of the configured language-switch trigger key,
/// or `None` if the configured key name could not be resolved.
pub fn switch_key_vk() -> Option<u32> {
    settings()
        .lock()
        .expect("capslock settings mutex poisoned")
        .switch_key_vk
}

pub fn set_paused(paused: bool) {
    let mut settings = settings().lock().expect("capslock settings mutex poisoned");
    settings.paused = paused;
}

pub fn handle_caps_lock_keydown(modifiers: ModifierState, process_name: Option<&str>) -> bool {
    let settings = *settings().lock().expect("capslock settings mutex poisoned");

    if settings.paused {
        return false;
    }

    if exclusions::is_module_excluded(ModuleId::CapsLock, process_name) {
        return false;
    }

    // The "real Caps Lock" modifier combo only makes sense when Caps Lock is the
    // configured trigger key; for any other trigger it must not pass the key
    // through as a Caps Lock.
    if settings.switch_key_vk == Some(VK_CAPITAL.0 as u32)
        && is_real_caps_combo(settings.real_caps_combo, modifiers)
    {
        return false;
    }

    if modifiers.any() {
        return false;
    }

    send_layout_switch_hotkey();

    true
}

/// Sends the system layout-switch hotkey (Left Win + Space) via SendInput.
/// Order matters: Space is pressed and released while Win is held down,
/// then Win is released last.
/// Injected events carry LLKHF_INJECTED, so our own hook ignores them.
fn send_layout_switch_hotkey() {
    let inputs = [
        keys::press(VK_LWIN, false),  // Win   down
        keys::press(VK_SPACE, false), // Space down
        keys::press(VK_SPACE, true),  // Space up
        keys::press(VK_LWIN, true),   // Win   up
    ];

    let sent = keys::send_inputs(&inputs);
    if sent != inputs.len() as u32 {
        log::warn!(
            "layout-switch hotkey: SendInput sent only {}/{} events",
            sent,
            inputs.len()
        );
    } else {
        log::debug!("layout-switch hotkey: SendInput sent {}/{} events", sent, inputs.len());
    }
}

fn is_real_caps_combo(combo: RealCapsCombo, modifiers: ModifierState) -> bool {
    match combo {
        RealCapsCombo::ShiftCaps => modifiers.shift,
        RealCapsCombo::AltCaps => modifiers.alt,
        RealCapsCombo::CtrlCaps => modifiers.ctrl,
    }
}

fn settings() -> &'static Mutex<RuntimeSettings> {
    SETTINGS.get_or_init(|| Mutex::new(RuntimeSettings::default()))
}
