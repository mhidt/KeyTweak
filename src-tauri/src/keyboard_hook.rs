use crate::{autoreplace, capslock, key_remap, translate};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
            VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
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
    if matches!(vk_code, code if code == VK_SHIFT.0 as u32 || code == VK_LSHIFT.0 as u32 || code == VK_RSHIFT.0 as u32)
    {
        SHIFT_DOWN.store(pressed, Ordering::Relaxed);
    } else if matches!(vk_code, code if code == VK_CONTROL.0 as u32 || code == VK_LCONTROL.0 as u32 || code == VK_RCONTROL.0 as u32)
    {
        CTRL_DOWN.store(pressed, Ordering::Relaxed);
    } else if matches!(vk_code, code if code == VK_MENU.0 as u32 || code == VK_LMENU.0 as u32 || code == VK_RMENU.0 as u32)
    {
        ALT_DOWN.store(pressed, Ordering::Relaxed);
    } else if matches!(vk_code, code if code == VK_LWIN.0 as u32 || code == VK_RWIN.0 as u32) {
        WIN_DOWN.store(pressed, Ordering::Relaxed);
    }
}
