use std::{
    env, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

const APP_NAME: &str = "KeyTweak";
const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const LAYERS_KEY_PATH: &str =
    r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
/// Value written to the AppCompatFlags layer that makes Windows always prompt
/// for elevation (UAC) when the executable is launched.
const RUN_AS_ADMIN_LAYER: &str = "~ RUNASADMIN";

#[derive(Debug, Error)]
pub enum AutoStartError {
    #[error("failed to locate executable for Windows startup: {0}")]
    ExecutablePath(io::Error),
    #[error("failed to update Windows startup registry entry: {0}")]
    Registry(io::Error),
}

pub type Result<T> = std::result::Result<T, AutoStartError>;

pub fn set_auto_start(enabled: bool) -> Result<()> {
    if enabled {
        let exe_path = startup_exe_path()?;
        set_run_entry(&exe_path)
    } else {
        delete_run_entry()
    }
}

fn startup_exe_path() -> Result<PathBuf> {
    let exe_path = env::current_exe().map_err(AutoStartError::ExecutablePath)?;

    #[cfg(debug_assertions)]
    {
        if let Some(release_path) = release_exe_path(&exe_path) {
            if release_path.exists() {
                return Ok(release_path);
            }
        }
    }

    Ok(exe_path)
}

#[cfg(debug_assertions)]
fn release_exe_path(exe_path: &Path) -> Option<PathBuf> {
    let file_name = exe_path.file_name()?;
    let target_dir = exe_path.parent()?.parent()?;
    Some(target_dir.join("release").join(file_name))
}

pub fn is_auto_start() -> Result<bool> {
    run_entry_exists()
}

/// Marks the current executable to always run elevated (UAC prompt on every
/// launch) by writing the AppCompatFlags "RUNASADMIN" layer, or removes it.
pub fn set_run_as_admin(enabled: bool) -> Result<()> {
    let exe_path = startup_exe_path()?;
    if enabled {
        set_run_as_admin_entry(&exe_path)
    } else {
        delete_run_as_admin_entry(&exe_path)
    }
}

#[cfg(windows)]
mod registry {
    use super::*;
    use std::slice;
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::PCWSTR,
        Win32::System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE,
            REG_SAM_FLAGS, REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteValueW,
            RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
        },
    };

    fn wide_null(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
        value
            .as_ref()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn reg_err(status: windows::Win32::Foundation::WIN32_ERROR) -> AutoStartError {
        AutoStartError::Registry(io::Error::from_raw_os_error(status.0 as i32))
    }

    fn is_not_found(code: u32) -> bool {
        code == 2 || code == 3
    }

    fn open_run_key(access: REG_SAM_FLAGS) -> Result<Option<HKEY>> {
        open_key(RUN_KEY_PATH, access)
    }

    /// Opens an HKCU subkey for the given path, returning `None` if it does not
    /// exist. Shared by the Run-entry and AppCompatFlags-layer helpers.
    fn open_key(path: &str, access: REG_SAM_FLAGS) -> Result<Option<HKEY>> {
        let key_path = wide_null(path);
        let mut key = HKEY::default();

        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(key_path.as_ptr()),
                0,
                access,
                &mut key,
            )
        };
        if is_not_found(status.0) {
            return Ok(None);
        }
        if status.0 != 0 {
            return Err(reg_err(status));
        }

        Ok(Some(key))
    }

    pub fn set_run_entry(exe_path: &Path) -> Result<()> {
        let key_path = wide_null(RUN_KEY_PATH);
        let value_name = wide_null(APP_NAME);
        let command = wide_null(&format!("\"{}\"", exe_path.display()));
        let bytes = unsafe {
            slice::from_raw_parts(command.as_ptr().cast::<u8>(), command.len() * size_of::<u16>())
        };
        let mut key = HKEY::default();

        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(key_path.as_ptr()),
                0,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
        };
        if status.0 != 0 {
            return Err(reg_err(status));
        }

        let status = unsafe {
            RegSetValueExW(key, PCWSTR(value_name.as_ptr()), 0, REG_SZ, Some(bytes))
        };
        unsafe { let _ = RegCloseKey(key); }

        if status.0 == 0 {
            Ok(())
        } else {
            Err(reg_err(status))
        }
    }

    pub fn delete_run_entry() -> Result<()> {
        let Some(key) = open_run_key(KEY_SET_VALUE)? else {
            return Ok(());
        };
        let value_name = wide_null(APP_NAME);

        let status = unsafe { RegDeleteValueW(key, PCWSTR(value_name.as_ptr())) };
        unsafe { let _ = RegCloseKey(key); }

        if status.0 == 0 || is_not_found(status.0) {
            Ok(())
        } else {
            Err(reg_err(status))
        }
    }

    pub fn run_entry_exists() -> Result<bool> {
        let Some(key) = open_run_key(KEY_QUERY_VALUE)? else {
            return Ok(false);
        };
        let value_name = wide_null(APP_NAME);

        let status = unsafe {
            RegQueryValueExW(key, PCWSTR(value_name.as_ptr()), None, None, None, None)
        };
        unsafe { let _ = RegCloseKey(key); }

        if status.0 == 0 {
            Ok(true)
        } else if is_not_found(status.0) {
            Ok(false)
        } else {
            Err(reg_err(status))
        }
    }

    pub fn set_run_as_admin_entry(exe_path: &Path) -> Result<()> {
        let key_path = wide_null(LAYERS_KEY_PATH);
        let value_name = wide_null(exe_path.as_os_str());
        let layer = wide_null(RUN_AS_ADMIN_LAYER);
        let bytes = unsafe {
            slice::from_raw_parts(layer.as_ptr().cast::<u8>(), layer.len() * size_of::<u16>())
        };
        let mut key = HKEY::default();

        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(key_path.as_ptr()),
                0,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
        };
        if status.0 != 0 {
            return Err(reg_err(status));
        }

        let status = unsafe {
            RegSetValueExW(key, PCWSTR(value_name.as_ptr()), 0, REG_SZ, Some(bytes))
        };
        unsafe { let _ = RegCloseKey(key); }

        if status.0 == 0 {
            Ok(())
        } else {
            Err(reg_err(status))
        }
    }

    pub fn delete_run_as_admin_entry(exe_path: &Path) -> Result<()> {
        let Some(key) = open_key(LAYERS_KEY_PATH, KEY_SET_VALUE)? else {
            return Ok(());
        };
        let value_name = wide_null(exe_path.as_os_str());

        let status = unsafe { RegDeleteValueW(key, PCWSTR(value_name.as_ptr())) };
        unsafe { let _ = RegCloseKey(key); }

        if status.0 == 0 || is_not_found(status.0) {
            Ok(())
        } else {
            Err(reg_err(status))
        }
    }
}

#[cfg(windows)]
use registry::{
    delete_run_as_admin_entry, delete_run_entry, run_entry_exists, set_run_as_admin_entry,
    set_run_entry,
};

#[cfg(not(windows))]
fn set_run_entry(_exe_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn delete_run_entry() -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn run_entry_exists() -> Result<bool> {
    Ok(false)
}

#[cfg(not(windows))]
fn set_run_as_admin_entry(_exe_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn delete_run_as_admin_entry(_exe_path: &Path) -> Result<()> {
    Ok(())
}
