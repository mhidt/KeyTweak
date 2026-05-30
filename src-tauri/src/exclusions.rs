use crate::config::{ExceptionMode, ModuleId, ProgramException};
use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::OsStringExt,
    path::Path,
    sync::{Mutex, OnceLock},
};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

#[derive(Debug, Clone)]
struct ExclusionsConfig {
    exception_mode: ExceptionMode,
    exceptions: Vec<ProgramException>,
}

impl Default for ExclusionsConfig {
    fn default() -> Self {
        Self {
            exception_mode: ExceptionMode::Blacklist,
            exceptions: Vec::new(),
        }
    }
}

static CONFIG: OnceLock<Mutex<ExclusionsConfig>> = OnceLock::new();

pub fn configure(exception_mode: ExceptionMode, exceptions: &[ProgramException]) {
    let mut config = config_store()
        .lock()
        .expect("exclusions config mutex poisoned");
    *config = ExclusionsConfig {
        exception_mode,
        exceptions: exceptions.to_vec(),
    };
}

/// Check if a module is excluded for the current foreground window.
/// `process_name` should be the lowercased filename of the foreground process
/// (obtained once per keyboard_proc call and passed in).
pub fn is_module_excluded(module: ModuleId, process_name: Option<&str>) -> bool {
    let Some(process_name) = process_name else {
        return false;
    };

    let config = config_store()
        .lock()
        .expect("exclusions config mutex poisoned");

    let in_list = config.exceptions.iter().any(|entry| {
        let module_matches = entry
            .modules
            .as_ref()
            .map_or(true, |mods| mods.contains(&module));
        module_matches && same_process_name(&entry.program, process_name)
    });

    match config.exception_mode {
        ExceptionMode::Blacklist => in_list,
        ExceptionMode::Whitelist => !in_list,
    }
}

/// Get the lowercased filename of the foreground window's process.
/// Call this once per keyboard_proc invocation and pass the result to modules.
pub fn foreground_process_name() -> Option<String> {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_invalid() {
        return None;
    }

    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(foreground, Some(&mut process_id)) };
    if process_id == 0 {
        return None;
    }

    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let mut buffer = [0u16; 32768];
    let mut size = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }

    result.ok()?;

    let path = OsString::from_wide(&buffer[..size as usize]);
    filename_lower(&path)
}

fn filename_lower(path: &OsStr) -> Option<String> {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
}

fn same_process_name(configured: &str, actual: &str) -> bool {
    let configured =
        filename_lower(OsStr::new(configured)).unwrap_or_else(|| configured.to_lowercase());
    configured == actual
}

/// Returns `true` if a configured program name/path matches the lowercased
/// filename of the actual foreground process. Reused by features that keep
/// their own per-item program lists (e.g. per-replacement auto-replace
/// exclusions).
pub fn program_matches(configured: &str, actual: &str) -> bool {
    same_process_name(configured, actual)
}

fn config_store() -> &'static Mutex<ExclusionsConfig> {
    CONFIG.get_or_init(|| Mutex::new(ExclusionsConfig::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_name_matching_uses_file_name() {
        assert!(same_process_name("code.exe", "code.exe"));
        assert!(same_process_name(
            r"C:\Program Files\App\code.exe",
            "code.exe"
        ));
        assert!(!same_process_name("notepad.exe", "code.exe"));
    }
}
