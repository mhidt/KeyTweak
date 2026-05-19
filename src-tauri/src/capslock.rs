use crate::{
    config::{CapsLockConfig, RealCapsCombo, SwitchMode},
    keyboard_hook::ModifierState,
};
use std::{
    ffi::c_void,
    sync::{Mutex, OnceLock},
};
use thiserror::Error;
use windows::Win32::{
    UI::Input::KeyboardAndMouse::{
        ActivateKeyboardLayout, GetKeyboardLayout, GetKeyboardLayoutList, HKL, KLF_REORDER,
    },
    UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, WM_INPUTLANGCHANGEREQUEST,
    },
};

#[derive(Debug, Clone, Copy)]
struct RuntimeSettings {
    switch_mode: SwitchMode,
    real_caps_combo: RealCapsCombo,
    paused: bool,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        let config = CapsLockConfig::default();

        Self {
            switch_mode: config.switch_mode,
            real_caps_combo: config.real_caps_combo,
            paused: config.paused,
        }
    }
}

#[derive(Debug, Default)]
struct LayoutState {
    previous_layout: Option<usize>,
}

#[derive(Debug, Error)]
pub enum CapsLockError {
    #[error("no keyboard layouts are installed")]
    NoLayouts,
    #[error("failed to activate keyboard layout")]
    Activate,
}

static SETTINGS: OnceLock<Mutex<RuntimeSettings>> = OnceLock::new();
static LAYOUT_STATE: OnceLock<Mutex<LayoutState>> = OnceLock::new();

pub fn configure(config: &CapsLockConfig) {
    let mut settings = settings().lock().expect("capslock settings mutex poisoned");
    *settings = RuntimeSettings {
        switch_mode: config.switch_mode,
        real_caps_combo: config.real_caps_combo,
        paused: config.paused,
    };
}

pub fn set_paused(paused: bool) {
    let mut settings = settings().lock().expect("capslock settings mutex poisoned");
    settings.paused = paused;
}

pub fn handle_caps_lock_keydown(modifiers: ModifierState) -> bool {
    let settings = *settings().lock().expect("capslock settings mutex poisoned");

    if settings.paused {
        return false;
    }

    if is_real_caps_combo(settings.real_caps_combo, modifiers) {
        return false;
    }

    if modifiers.any() {
        return false;
    }

    if let Err(error) = switch_layout(settings.switch_mode) {
        log::error!("failed to switch keyboard layout: {error}");
    }

    true
}

fn is_real_caps_combo(combo: RealCapsCombo, modifiers: ModifierState) -> bool {
    match combo {
        RealCapsCombo::ShiftCaps => modifiers.shift,
        RealCapsCombo::AltCaps => modifiers.alt,
        RealCapsCombo::CtrlCaps => modifiers.ctrl,
    }
}

fn switch_layout(mode: SwitchMode) -> Result<(), CapsLockError> {
    match mode {
        SwitchMode::Previous => switch_previous_layout(),
        SwitchMode::Default => switch_next_layout(),
    }
}

fn switch_previous_layout() -> Result<(), CapsLockError> {
    let current = current_layout_id();
    let previous = {
        let state = layout_state()
            .lock()
            .expect("capslock layout state mutex poisoned");
        state.previous_layout
    };

    let target = match previous {
        Some(previous) if previous != current => previous,
        _ => next_layout_id(current)?,
    };

    activate_layout(target)?;

    let mut state = layout_state()
        .lock()
        .expect("capslock layout state mutex poisoned");
    state.previous_layout = Some(current);

    Ok(())
}

fn switch_next_layout() -> Result<(), CapsLockError> {
    let current = current_layout_id();
    let target = next_layout_id(current)?;

    activate_layout(target)?;

    let mut state = layout_state()
        .lock()
        .expect("capslock layout state mutex poisoned");
    state.previous_layout = Some(current);

    Ok(())
}

fn current_layout_id() -> usize {
    let foreground = unsafe { GetForegroundWindow() };
    let thread_id = if foreground.is_invalid() {
        0
    } else {
        unsafe { GetWindowThreadProcessId(foreground, None) }
    };

    unsafe { GetKeyboardLayout(thread_id).0 as usize }
}

fn next_layout_id(current: usize) -> Result<usize, CapsLockError> {
    let layouts = installed_layouts();

    if layouts.is_empty() {
        return Err(CapsLockError::NoLayouts);
    }

    let current_index = layouts
        .iter()
        .position(|layout| *layout == current)
        .unwrap_or(0);
    let next_index = (current_index + 1) % layouts.len();

    Ok(layouts[next_index])
}

fn installed_layouts() -> Vec<usize> {
    let count = unsafe { GetKeyboardLayoutList(None) };

    if count <= 0 {
        return Vec::new();
    }

    let mut layouts = vec![HKL(std::ptr::null_mut()); count as usize];
    let actual = unsafe { GetKeyboardLayoutList(Some(&mut layouts)) };

    layouts
        .into_iter()
        .take(actual.max(0) as usize)
        .map(|layout| layout.0 as usize)
        .collect()
}

fn activate_layout(layout: usize) -> Result<(), CapsLockError> {
    let hkl = HKL(layout as *mut c_void);

    unsafe { ActivateKeyboardLayout(hkl, KLF_REORDER) }
        .map(|_| ())
        .map_err(|_| CapsLockError::Activate)?;

    let foreground = unsafe { GetForegroundWindow() };
    if !foreground.is_invalid() {
        let _ = unsafe {
            PostMessageW(
                foreground,
                WM_INPUTLANGCHANGEREQUEST,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(layout as isize),
            )
        };
    }

    Ok(())
}

fn settings() -> &'static Mutex<RuntimeSettings> {
    SETTINGS.get_or_init(|| Mutex::new(RuntimeSettings::default()))
}

fn layout_state() -> &'static Mutex<LayoutState> {
    LAYOUT_STATE.get_or_init(|| Mutex::new(LayoutState::default()))
}
