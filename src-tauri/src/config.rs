use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

const CONFIG_DIR_NAME: &str = "KeyTweak";
const CONFIG_FILE_NAME: &str = "config.json";
const TEMP_CONFIG_FILE_NAME: &str = "config.json.tmp";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub caps_lock: CapsLockConfig,
    #[serde(default)]
    pub auto_replace: AutoReplaceConfig,
    #[serde(default)]
    pub key_remap: KeyRemapConfig,
    #[serde(default)]
    pub translate: TranslateConfig,
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub exception_mode: ExceptionMode,
    #[serde(default)]
    pub exceptions: Vec<ProgramException>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            caps_lock: CapsLockConfig::default(),
            auto_replace: AutoReplaceConfig::default(),
            key_remap: KeyRemapConfig::default(),
            translate: TranslateConfig::default(),
            general: GeneralConfig::default(),
            exception_mode: ExceptionMode::default(),
            exceptions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapsLockConfig {
    #[serde(default)]
    pub switch_mode: SwitchMode,
    #[serde(default)]
    pub real_caps_combo: RealCapsCombo,
    #[serde(default = "default_true")]
    pub auto_start: bool,
    #[serde(default)]
    pub paused: bool,
}

impl Default for CapsLockConfig {
    fn default() -> Self {
        Self {
            switch_mode: SwitchMode::Previous,
            real_caps_combo: RealCapsCombo::ShiftCaps,
            auto_start: true,
            paused: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwitchMode {
    Previous,
    Default,
}

impl Default for SwitchMode {
    fn default() -> Self {
        Self::Previous
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealCapsCombo {
    ShiftCaps,
    AltCaps,
    CtrlCaps,
}

impl Default for RealCapsCombo {
    fn default() -> Self {
        Self::ShiftCaps
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoReplaceConfig {
    #[serde(default = "default_true")]
    pub trigger_space: bool,
    #[serde(default = "default_true")]
    pub trigger_tab: bool,
    #[serde(default)]
    pub trigger_enter: bool,
    #[serde(default = "default_true")]
    pub trigger_punctuation: bool,
    #[serde(default = "default_true")]
    pub whole_words_only: bool,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub replacements: Vec<Replacement>,
}

impl Default for AutoReplaceConfig {
    fn default() -> Self {
        Self {
            trigger_space: true,
            trigger_tab: true,
            trigger_enter: false,
            trigger_punctuation: true,
            whole_words_only: true,
            case_sensitive: false,
            replacements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    pub short: String,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramException {
    pub program: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<ModuleId>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleId {
    CapsLock,
    AutoReplace,
    KeyRemap,
    Translate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRemapConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub mappings: Vec<KeyRemap>,
}

impl Default for KeyRemapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mappings: vec![KeyRemap {
                from: "left_alt".to_string(),
                to: "win".to_string(),
                enabled: false,
            }],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRemap {
    pub from: String,
    pub to: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionMode {
    Blacklist,
    Whitelist,
}

impl Default for ExceptionMode {
    fn default() -> Self {
        Self::Blacklist
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslateConfig {
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_true")]
    pub auto_detect_language: bool,
    #[serde(default = "default_target_language")]
    pub target_language: String,
    #[serde(default = "default_translate_hotkey")]
    pub hotkey_translate: String,
    #[serde(default = "default_reverse_hotkey")]
    pub hotkey_reverse: String,
}

impl Default for TranslateConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            api_key: String::new(),
            auto_detect_language: true,
            target_language: default_target_language(),
            hotkey_translate: default_translate_hotkey(),
            hotkey_reverse: default_reverse_hotkey(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_app_language")]
    pub app_language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            app_language: default_app_language(),
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to locate a user config directory")]
    ConfigDirectoryUnavailable,
    #[error("failed to create config directory at {path}: {source}")]
    CreateDirectory { path: PathBuf, source: io::Error },
    #[error("failed to read config file at {path}: {source}")]
    ReadFile { path: PathBuf, source: io::Error },
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to parse config file at {path}: {source}")]
    ParseFile {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("failed to write temp config file at {path}: {source}")]
    WriteTempFile { path: PathBuf, source: io::Error },
    #[error("failed to atomically replace config file at {path}: {source}")]
    ReplaceFile { path: PathBuf, source: io::Error },
}

pub type Result<T> = std::result::Result<T, ConfigError>;

pub fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or(ConfigError::ConfigDirectoryUnavailable)?;
    Ok(base.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME))
}

pub fn load_config() -> Result<Config> {
    load_config_from_path(config_path()?)
}

#[allow(dead_code)]
pub fn save_config(config: &Config) -> Result<()> {
    save_config_to_path(config_path()?, config)
}

pub fn load_config_from_path(path: impl AsRef<Path>) -> Result<Config> {
    let path = path.as_ref();

    if !path.exists() {
        let config = Config::default();
        save_config_to_path(path, &config)?;
        return Ok(config);
    }

    let raw = fs::read_to_string(path).map_err(|source| ConfigError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let mut config: Config = serde_json::from_str(&raw).map_err(|source| ConfigError::ParseFile {
        path: path.to_path_buf(),
        source,
    })?;
    migrate_config(&mut config);
    Ok(config)
}

pub fn save_config_to_path(path: impl AsRef<Path>, config: &Config) -> Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let temp_path = path
        .parent()
        .map(|parent| parent.join(TEMP_CONFIG_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(TEMP_CONFIG_FILE_NAME));
    let bytes = serde_json::to_vec_pretty(config)?;

    fs::write(&temp_path, bytes).map_err(|source| ConfigError::WriteTempFile {
        path: temp_path.clone(),
        source,
    })?;

    replace_file(&temp_path, path).map_err(|source| ConfigError::ReplaceFile {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(windows)]
fn replace_file(temp_path: &Path, target_path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::PCWSTR,
        Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        },
    };

    let temp_wide: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let target_wide: Vec<u16> = target_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        MoveFileExW(
            PCWSTR(temp_wide.as_ptr()),
            PCWSTR(target_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|_| io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn replace_file(temp_path: &Path, target_path: &Path) -> io::Result<()> {
    fs::rename(temp_path, target_path)
}

fn default_true() -> bool {
    true
}

fn default_target_language() -> String {
    "ru".to_string()
}

fn default_server_url() -> String {
    "http://127.0.0.1:5000".to_string()
}

fn default_translate_hotkey() -> String {
    "ctrl+c+c".to_string()
}

fn default_reverse_hotkey() -> String {
    "ctrl+shift+c".to_string()
}

fn default_app_language() -> String {
    "en".to_string()
}

fn default_theme() -> String {
    "system".to_string()
}

fn migrate_config(config: &mut Config) {
    for mapping in &mut config.key_remap.mappings {
        if mapping.from.eq_ignore_ascii_case("alt+v") && mapping.to.eq_ignore_ascii_case("win+v") {
            mapping.from = "left_alt".to_string();
            mapping.to = "win".to_string();
            mapping.enabled = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrips_json() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).expect("serialize default config");
        let parsed: Config = serde_json::from_str(&json).expect("deserialize default config");

        assert_eq!(parsed, config);
    }

    #[test]
    fn save_and_load_config_from_disk() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let path = temp_dir.path().join("KeyTweak").join("config.json");
        let config = Config::default();

        save_config_to_path(&path, &config).expect("save config");
        let loaded = load_config_from_path(&path).expect("load config");

        assert_eq!(loaded, config);
        assert!(path.exists());
    }

    #[test]
    fn save_config_replaces_existing_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let path = temp_dir.path().join("KeyTweak").join("config.json");
        let original = Config::default();
        let mut updated = Config::default();
        updated.caps_lock.switch_mode = SwitchMode::Default;
        updated.translate.target_language = "en".to_string();

        save_config_to_path(&path, &original).expect("save original config");
        save_config_to_path(&path, &updated).expect("replace config");
        let loaded = load_config_from_path(&path).expect("load replaced config");

        assert_eq!(loaded, updated);
    }

    #[test]
    fn missing_config_is_created_with_defaults() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let path = temp_dir.path().join("KeyTweak").join("config.json");

        let loaded = load_config_from_path(&path).expect("load missing config");

        assert_eq!(loaded, Config::default());
        assert!(path.exists());
    }
}
