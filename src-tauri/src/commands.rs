use crate::{autostart, config, config::Config, state::AppState, toast, translate, tray};
use tauri::{AppHandle, State};

use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::CloseHandle,
        Storage::FileSystem::{
            GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
        },
        System::Threading::{
            GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowThreadProcessId,
            IsWindowVisible,
        },
    },
};

#[derive(Serialize, Clone)]
pub struct RunningProgram {
    pub exe_name: String,
    pub display_name: String,
}

/// Get display name from exe's version info resource (FileDescription, fallback to ProductName)
fn get_file_description(full_path: &str) -> Option<String> {
    let path_wide: Vec<u16> = full_path.encode_utf16().chain(std::iter::once(0)).collect();
    let path_pcwstr = windows::core::PCWSTR(path_wide.as_ptr());

    unsafe {
        let size = GetFileVersionInfoSizeW(path_pcwstr, None);
        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        if GetFileVersionInfoW(path_pcwstr, 0, size, buffer.as_mut_ptr() as *mut _).is_err() {
            return None;
        }

        // Try FileDescription first, then ProductName as fallback
        let fields = &["FileDescription", "ProductName"];

        for field in fields {
            if let Some(value) = query_version_string(&buffer, field) {
                return Some(value);
            }
        }
    }

    None
}

/// Query a string value from version info buffer, trying multiple language/codepage combos
unsafe fn query_version_string(buffer: &[u8], field: &str) -> Option<String> {
    // Try common language/codepage combinations
    let lang_codepages: &[&str] = &[
        "040904B0", // US English, Unicode
        "040904E4", // US English, Latin-1
        "041904B0", // Russian, Unicode
        "000004B0", // Language-neutral, Unicode
    ];

    for lang_cp in lang_codepages {
        let sub_block = format!("\\StringFileInfo\\{}\\{}\0", lang_cp, field);
        let sub_block_wide: Vec<u16> = sub_block.encode_utf16().collect();

        let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len: u32 = 0;

        if VerQueryValueW(
            buffer.as_ptr() as *const _,
            windows::core::PCWSTR(sub_block_wide.as_ptr()),
            &mut ptr,
            &mut len,
        )
        .as_bool()
            && len > 0
            && !ptr.is_null()
        {
            let slice = std::slice::from_raw_parts(ptr as *const u16, len as usize);
            let desc = String::from_utf16_lossy(slice)
                .trim_end_matches('\0')
                .to_string();
            if !desc.is_empty() {
                return Some(desc);
            }
        }
    }

    // Fallback: try to find any translation and use it
    let translation_block = "\\VarFileInfo\\Translation\0";
    let translation_wide: Vec<u16> = translation_block.encode_utf16().collect();
    let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut len: u32 = 0;

    if VerQueryValueW(
        buffer.as_ptr() as *const _,
        windows::core::PCWSTR(translation_wide.as_ptr()),
        &mut ptr,
        &mut len,
    )
    .as_bool()
        && len >= 4
        && !ptr.is_null()
    {
        let lang = *(ptr as *const u16);
        let codepage = *((ptr as *const u16).add(1));
        let sub_block = format!(
            "\\StringFileInfo\\{:04X}{:04X}\\{}\0",
            lang, codepage, field
        );
        let sub_block_wide: Vec<u16> = sub_block.encode_utf16().collect();

        let mut desc_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut desc_len: u32 = 0;

        if VerQueryValueW(
            buffer.as_ptr() as *const _,
            windows::core::PCWSTR(sub_block_wide.as_ptr()),
            &mut desc_ptr,
            &mut desc_len,
        )
        .as_bool()
            && desc_len > 0
            && !desc_ptr.is_null()
        {
            let slice = std::slice::from_raw_parts(desc_ptr as *const u16, desc_len as usize);
            let desc = String::from_utf16_lossy(slice)
                .trim_end_matches('\0')
                .to_string();
            if !desc.is_empty() {
                return Some(desc);
            }
        }
    }

    None
}

struct EnumWindowsData {
    /// exe_name -> full_path
    programs: HashMap<String, String>,
}

unsafe extern "system" fn enum_windows_callback(
    hwnd: windows::Win32::Foundation::HWND,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::BOOL {
    let data = &mut *(lparam.0 as *mut EnumWindowsData);

    if !IsWindowVisible(hwnd).as_bool() {
        return windows::Win32::Foundation::BOOL(1);
    }

    let title_len = GetWindowTextLengthW(hwnd);
    if title_len == 0 {
        return windows::Win32::Foundation::BOOL(1);
    }

    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    if process_id == 0 || process_id == GetCurrentProcessId() {
        return windows::Win32::Foundation::BOOL(1);
    }

    // Get process path
    let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) else {
        return windows::Win32::Foundation::BOOL(1);
    };
    let mut name_buf = [0u16; 32768];
    let mut name_size = name_buf.len() as u32;
    let name_result = QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_WIN32,
        PWSTR(name_buf.as_mut_ptr()),
        &mut name_size,
    );
    let _ = CloseHandle(process);
    if name_result.is_err() {
        return windows::Win32::Foundation::BOOL(1);
    }

    let full_path = String::from_utf16_lossy(&name_buf[..name_size as usize]);
    let exe_name = full_path
        .rsplit('\\')
        .next()
        .unwrap_or(&full_path)
        .to_lowercase();

    // Skip if we already have this exe
    if data.programs.contains_key(&exe_name) {
        return windows::Win32::Foundation::BOOL(1);
    }

    data.programs.insert(exe_name, full_path);

    windows::Win32::Foundation::BOOL(1)
}

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config()
}

#[tauri::command]
pub fn set_config(cfg: Config, state: State<'_, AppState>) -> CommandResult<()> {
    config::save_config(&cfg).map_err(|error| error.to_string())?;
    state.set_config(cfg);
    Ok(())
}

#[tauri::command]
pub fn pause_caps_lock(
    paused: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    state.set_caps_paused(paused);
    tray::rebuild_tray_menu(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn is_caps_paused(state: State<'_, AppState>) -> bool {
    state.caps_paused()
}

#[tauri::command]
pub fn set_auto_start(enabled: bool) -> CommandResult<()> {
    autostart::set_auto_start(enabled).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn is_auto_start() -> CommandResult<bool> {
    autostart::is_auto_start().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn test_translate_api(
    server_url: String,
    api_key: String,
    target: String,
) -> CommandResult<String> {
    translate::test_translate_api(&server_url, &api_key, &target)
}

#[tauri::command]
pub fn replace_with_translation(text: String) -> CommandResult<()> {
    translate::replace_with_translation(text)
}

#[tauri::command]
pub fn copy_to_clipboard(text: String) -> CommandResult<()> {
    translate::copy_to_clipboard(text)
}

#[tauri::command]
pub fn hide_translation_toast() {
    toast::hide_translation_toast();
}

#[tauri::command]
pub fn export_replacements_json(json: String) -> CommandResult<bool> {
    let dialog = rfd::FileDialog::new()
        .set_title("Экспорт замен")
        .set_file_name("keytweak-replacements.json")
        .add_filter("JSON", &["json"]);

    match dialog.save_file() {
        Some(path) => {
            fs::write(&path, json.as_bytes()).map_err(|e| e.to_string())?;
            Ok(true)
        }
        None => Ok(false), // user cancelled
    }
}

#[tauri::command]
pub fn import_replacements_json() -> CommandResult<Option<String>> {
    let dialog = rfd::FileDialog::new()
        .set_title("Импорт замен")
        .add_filter("JSON", &["json"]);

    match dialog.pick_file() {
        Some(path) => {
            let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            Ok(Some(content))
        }
        None => Ok(None), // user cancelled
    }
}

#[tauri::command]
pub fn get_running_programs() -> Vec<RunningProgram> {
    let mut data = EnumWindowsData {
        programs: HashMap::new(),
    };
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_callback),
            windows::Win32::Foundation::LPARAM(&mut data as *mut _ as isize),
        );
    }

    let mut programs: Vec<RunningProgram> = data
        .programs
        .into_iter()
        .map(|(exe_name, full_path)| {
            let display_name = get_file_description(&full_path).unwrap_or_default();
            RunningProgram { exe_name, display_name }
        })
        .collect();

    programs.sort_by(|a, b| a.exe_name.cmp(&b.exe_name));
    programs
}

#[tauri::command]
pub fn pick_program_file() -> Option<String> {
    let dialog = rfd::FileDialog::new()
        .set_title("Выбрать программу")
        .add_filter("Приложения", &["exe"]);

    dialog.pick_file().and_then(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
    })
}
