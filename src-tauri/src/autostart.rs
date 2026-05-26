use std::{io, path::Path};

use thiserror::Error;

const APP_NAME: &str = "KeyTweak";
const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[derive(Debug, Error)]
pub enum AutoStartError {
    #[error("failed to update Windows startup registry entry: {0}")]
    Registry(io::Error),
}

pub type Result<T> = std::result::Result<T, AutoStartError>;

pub fn set_auto_start(enabled: bool, exe_path: &Path) -> Result<()> {
    if enabled {
        set_run_entry(exe_path)
    } else {
        delete_run_entry()
    }
}

pub fn is_auto_start() -> Result<bool> {
    run_entry_exists()
}

#[cfg(windows)]
fn set_run_entry(exe_path: &Path) -> Result<()> {
    use std::slice;
    use windows::{
        core::PCWSTR,
        Win32::System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
            KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
        },
    };

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
        return Err(AutoStartError::Registry(io::Error::from_raw_os_error(
            status.0 as i32,
        )));
    }

    let status = unsafe {
        RegSetValueExW(key, PCWSTR(value_name.as_ptr()), 0, REG_SZ, Some(bytes))
    };
    unsafe {
        let _ = RegCloseKey(key);
    }

    if status.0 == 0 {
        Ok(())
    } else {
        Err(AutoStartError::Registry(io::Error::from_raw_os_error(
            status.0 as i32,
        )))
    }
}

#[cfg(windows)]
fn delete_run_entry() -> Result<()> {
    use windows::{
        core::PCWSTR,
        Win32::System::Registry::{
            RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
        },
    };

    let key_path = wide_null(RUN_KEY_PATH);
    let value_name = wide_null(APP_NAME);
    let mut key = HKEY::default();

    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
    };
    if is_not_found(status.0) {
        return Ok(());
    }
    if status.0 != 0 {
        return Err(AutoStartError::Registry(io::Error::from_raw_os_error(
            status.0 as i32,
        )));
    }

    let status = unsafe { RegDeleteValueW(key, PCWSTR(value_name.as_ptr())) };
    unsafe {
        let _ = RegCloseKey(key);
    }

    if status.0 == 0 || is_not_found(status.0) {
        Ok(())
    } else {
        Err(AutoStartError::Registry(io::Error::from_raw_os_error(
            status.0 as i32,
        )))
    }
}

#[cfg(windows)]
fn run_entry_exists() -> Result<bool> {
    use windows::{
        core::PCWSTR,
        Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
        },
    };

    let key_path = wide_null(RUN_KEY_PATH);
    let value_name = wide_null(APP_NAME);
    let mut key = HKEY::default();

    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        )
    };
    if is_not_found(status.0) {
        return Ok(false);
    }
    if status.0 != 0 {
        return Err(AutoStartError::Registry(io::Error::from_raw_os_error(
            status.0 as i32,
        )));
    }

    let status =
        unsafe { RegQueryValueExW(key, PCWSTR(value_name.as_ptr()), None, None, None, None) };
    unsafe {
        let _ = RegCloseKey(key);
    }

    if status.0 == 0 {
        Ok(true)
    } else if is_not_found(status.0) {
        Ok(false)
    } else {
        Err(AutoStartError::Registry(io::Error::from_raw_os_error(
            status.0 as i32,
        )))
    }
}

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

#[cfg(windows)]
fn wide_null(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn is_not_found(code: u32) -> bool {
    code == 2 || code == 3
}
