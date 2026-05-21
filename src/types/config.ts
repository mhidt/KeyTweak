export type SwitchMode = "previous" | "default";
export type RealCapsCombo = "shift_caps" | "alt_caps" | "ctrl_caps";
export type ExceptionMode = "blacklist" | "whitelist";

export interface Config {
  caps_lock: CapsLockConfig;
  auto_replace: AutoReplaceConfig;
  translate: TranslateConfig;
  general: GeneralConfig;
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
  exceptions: ProgramException[];
}

export interface Replacement {
  short: string;
  replacement: string;
}

export interface ProgramException {
  program: string;
  mode: ExceptionMode;
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
}
