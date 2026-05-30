export type SwitchMode = "previous" | "default";
export type RealCapsCombo = "shift_caps" | "alt_caps" | "ctrl_caps";
export type ExceptionMode = "blacklist" | "whitelist";
export type ThemeMode = "system" | "light" | "dark";

export interface Config {
  caps_lock: CapsLockConfig;
  auto_replace: AutoReplaceConfig;
  key_remap: KeyRemapConfig;
  translate: TranslateConfig;
  general: GeneralConfig;
  exception_mode: ExceptionMode;
  exceptions: ProgramException[];
}

export interface CapsLockConfig {
  switch_mode: SwitchMode;
  real_caps_combo: RealCapsCombo;
  auto_start: boolean;
  paused: boolean;
}

export interface AutoReplaceConfig {
  trigger_space: boolean;
  trigger_tab: boolean;
  trigger_enter: boolean;
  trigger_punctuation: boolean;
  whole_words_only: boolean;
  case_sensitive: boolean;
  replacements: Replacement[];
}

export interface Replacement {
  short: string;
  replacement: string;
}

export type ModuleId = "caps_lock" | "auto_replace" | "key_remap" | "translate";

export interface ProgramException {
  program: string;
  display_name?: string;
  modules?: ModuleId[];
}

export interface KeyRemapConfig {
  enabled: boolean;
  mappings: KeyRemap[];
}

export interface KeyRemap {
  from: string;
  to: string;
  enabled: boolean;
}

export interface TranslateConfig {
  server_url: string;
  api_key: string;
  auto_detect_language: boolean;
  target_language: string;
  hotkey_translate: string;
  hotkey_reverse: string;
}

export interface GeneralConfig {
  app_language: string;
  theme: ThemeMode;
}
