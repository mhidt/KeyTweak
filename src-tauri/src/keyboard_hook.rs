use crate::{autoreplace, capslock, key_remap, translate};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            VIRTUAL_KEY, VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
            VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK,
            KBDLLHOOKSTRUCT, LLKHF_INJECTED, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
            WM_SYSKEYUP,
        },
    },
};

static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
static ALT_DOWN: AtomicBool = AtomicBool::new(false);
static WIN_DOWN: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Error)]
pub enum KeyboardHookError {
    #[error("low-level keyboard hook is already installed")]
    AlreadyInstalled,
    #[error("failed to get current module handle")]
    ModuleHandle,
    #[error("failed to install low-level keyboard hook")]
    Install,
}

pub type Result<T> = std::result::Result<T, KeyboardHookError>;

pub struct KeyboardHook {
    handle: HHOOK,
}

// HHOOK is an OS handle. We store it behind AppState's Mutex and only use it
// for install/uninstall lifecycle management.
unsafe impl Send for KeyboardHook {}

impl KeyboardHook {
    pub fn install() -> Result<Self> {
        if HOOK_INSTALLED.swap(true, Ordering::SeqCst) {
            return Err(KeyboardHookError::AlreadyInstalled);
        }

        let module = unsafe { GetModuleHandleW(None) }.map_err(|_| {
            HOOK_INSTALLED.store(false, Ordering::SeqCst);
            KeyboardHookError::ModuleHandle
        })?;

        let handle = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), HINSTANCE(module.0), 0)
        }
        .map_err(|_| {
            HOOK_INSTALLED.store(false, Ordering::SeqCst);
            KeyboardHookError::Install
        })?;

        Ok(Self { handle })
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.handle);
        }
        HOOK_INSTALLED.store(false, Ordering::SeqCst);
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let is_keydown = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
        let is_keyup = wparam.0 as u32 == WM_KEYUP || wparam.0 as u32 == WM_SYSKEYUP;
        let is_injected = event.flags.contains(LLKHF_INJECTED);

        // On AltGr-capable keyboard layouts (e.g. Russian), Windows injects a
        // fake Left Ctrl event right before every Right Alt event. The fake Ctrl
        // has scanCode == 0x21D (real LCtrl has 0x1D). When Right Alt is being
        // remapped we must suppress this fake Ctrl so it doesn't leak through.
        if (is_keydown || is_keyup)
            && !is_injected
            && event.vkCode == VK_LCONTROL.0 as u32
            && event.scanCode == 0x21D
            && key_remap::has_right_alt_mapping()
        {
            return LRESULT(1);
        }

        if (is_keydown || is_keyup)
            && !is_injected
            && key_remap::handle_key_event(event.vkCode, is_keyup)
        {
            return LRESULT(1);
        }

        if (is_keydown || is_keyup) && !is_injected {
            update_modifier_state(event.vkCode, is_keydown);
        }

        if is_keydown && !is_injected {
            let modifiers = ModifierState::current();

            if event.vkCode == VK_CAPITAL.0 as u32 && capslock::handle_caps_lock_keydown(modifiers)
            {
                return LRESULT(1);
            }

            if translate::handle_keydown(event.vkCode, modifiers) {
                return LRESULT(1);
            }

            if autoreplace::handle_keydown(event.vkCode, event.scanCode, modifiers) {
                return LRESULT(1);
            }
        }
    }

    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub win: bool,
}

impl ModifierState {
    pub fn any(self) -> bool {
        self.shift || self.ctrl || self.alt || self.win
    }

    pub fn current() -> Self {
        Self {
            shift: SHIFT_DOWN.load(Ordering::Relaxed),
            ctrl: CTRL_DOWN.load(Ordering::Relaxed),
            alt: ALT_DOWN.load(Ordering::Relaxed),
            win: WIN_DOWN.load(Ordering::Relaxed),
        }
    }
}

fn update_modifier_state(vk_code: u32, pressed: bool) {
    let vk = VIRTUAL_KEY(vk_code as u16);

    if matches!(vk, VK_SHIFT | VK_LSHIFT | VK_RSHIFT) {
        SHIFT_DOWN.store(pressed, Ordering::Relaxed);
    } else if matches!(vk, VK_CONTROL | VK_LCONTROL | VK_RCONTROL) {
        CTRL_DOWN.store(pressed, Ordering::Relaxed);
    } else if matches!(vk, VK_MENU | VK_LMENU | VK_RMENU) {
        ALT_DOWN.store(pressed, Ordering::Relaxed);
    } else if matches!(vk, VK_LWIN | VK_RWIN) {
        WIN_DOWN.store(pressed, Ordering::Relaxed);
    }
}
