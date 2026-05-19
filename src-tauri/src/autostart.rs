use std::{ffi::OsStr, io, os::windows::ffi::OsStrExt, path::Path};
use thiserror::Error;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::ERROR_FILE_NOT_FOUND,
        System::Registry::{
            RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
            HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ,
        },
    },
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "KeyTweak";

#[derive(Debug, Error)]
pub enum AutoStartError {
    #[error("failed to open Windows Run registry key: {0}")]
    Open(io::Error),
    #[error("failed to update Windows Run registry key: {0}")]
    Write(io::Error),
    #[error("failed to read Windows Run registry key: {0}")]
    Read(io::Error),
}

pub type Result<T> = std::result::Result<T, AutoStartError>;

pub fn set_auto_start(enabled: bool, exe_path: &Path) -> Result<()> {
    let key = open_run_key(KEY_SET_VALUE).map_err(AutoStartError::Open)?;
    let value_name = wide(VALUE_NAME);

    let result = if enabled {
        let quoted_path = format!("\"{}\"", exe_path.display());
        let data = wide(&quoted_path);
        let bytes =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) };

        let status =
            unsafe { RegSetValueExW(key, PCWSTR(value_name.as_ptr()), 0, REG_SZ, Some(bytes)) };

        status
            .ok()
            .map_err(|_| AutoStartError::Write(io::Error::last_os_error()))
    } else {
        let status = unsafe { RegDeleteValueW(key, PCWSTR(value_name.as_ptr())) };

        if status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            status
                .ok()
                .map_err(|_| AutoStartError::Write(io::Error::last_os_error()))
        }
    };

    unsafe {
        let _ = RegCloseKey(key);
    }

    result
}

pub fn is_auto_start() -> Result<bool> {
    let key = open_run_key(KEY_QUERY_VALUE).map_err(AutoStartError::Open)?;
    let value_name = wide(VALUE_NAME);
    let status =
        unsafe { RegQueryValueExW(key, PCWSTR(value_name.as_ptr()), None, None, None, None) };

    unsafe {
        let _ = RegCloseKey(key);
    }

    if status == ERROR_FILE_NOT_FOUND {
        Ok(false)
    } else {
        status
            .ok()
            .map(|_| true)
            .map_err(|_| AutoStartError::Read(io::Error::last_os_error()))
    }
}

fn open_run_key(access: REG_SAM_FLAGS) -> io::Result<HKEY> {
    let subkey = wide(RUN_KEY);
    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            access,
            &mut key,
        )
    };

    status
        .ok()
        .map(|_| key)
        .map_err(|_| io::Error::last_os_error())
}

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
