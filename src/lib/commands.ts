import { invoke } from "@tauri-apps/api/core";
import type { Config } from "../types/config";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const defaultConfig: Config = {
  caps_lock: {
    switch_mode: "previous",
    switch_key: "capslock",
    real_caps_combo: "shift_caps",
    auto_start: false,
    paused: false,
  },
  auto_replace: {
    trigger_space: true,
    trigger_tab: true,
    trigger_enter: false,
    trigger_punctuation: true,
    whole_words_only: true,
    case_sensitive: false,
    replacements: [],
  },
  key_remap: {
    enabled: true,
    mappings: [{ from: "left_alt", to: "win", enabled: false }],
  },
  translate: {
    server_url: "http://127.0.0.1:5000",
    api_key: "",
    auto_detect_language: true,
    target_language: "ru",
    hotkey_translate: "ctrl+c+c",
    hotkey_reverse: "ctrl+shift+c",
  },
  general: {
    app_language: "en",
    theme: "system",
  },
  exception_mode: "blacklist",
  exceptions: [],
};

function inTauri() {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`IPC call timed out after ${ms}ms`)),
      ms,
    );
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

export function getConfig() {
  if (!inTauri()) return Promise.resolve(defaultConfig);
  return withTimeout(invoke<Config>("get_config"), 10000);
}

export function setConfig(cfg: Config) {
  if (!inTauri()) return Promise.resolve(void cfg);
  return invoke<void>("set_config", { cfg });
}

export function pauseCapsLock(paused: boolean) {
  if (!inTauri()) return Promise.resolve(void paused);
  return invoke<void>("pause_caps_lock", { paused });
}

export function isCapsPaused() {
  if (!inTauri()) return Promise.resolve(false);
  return invoke<boolean>("is_caps_paused");
}

export function setAutoStart(enabled: boolean) {
  if (!inTauri()) return Promise.resolve(void enabled);
  return invoke<void>("set_auto_start", { enabled });
}

export function isAutoStart() {
  if (!inTauri()) return Promise.resolve(defaultConfig.caps_lock.auto_start);
  return invoke<boolean>("is_auto_start");
}

export function testTranslateApi(
  serverUrl: string,
  apiKey: string,
  target: string,
) {
  if (!inTauri()) return Promise.resolve(target === "ru" ? "привет" : "hello");
  return invoke<string>("test_translate_api", { serverUrl, apiKey, target });
}

export function replaceWithTranslation(text: string) {
  if (!inTauri()) return Promise.resolve();
  return invoke<void>("replace_with_translation", { text });
}

export function copyToClipboard(text: string) {
  if (!inTauri())
    return navigator.clipboard?.writeText(text) ?? Promise.resolve();
  return invoke<void>("copy_to_clipboard", { text });
}

export function hideTranslationToast() {
  if (!inTauri()) return Promise.resolve();
  return invoke<void>("hide_translation_toast");
}

export function exportReplacementsJson(json: string) {
  if (!inTauri()) return Promise.resolve(false);
  return invoke<boolean>("export_replacements_json", { json });
}

export function importReplacementsJson() {
  if (!inTauri()) return Promise.resolve(null as string | null);
  return invoke<string | null>("import_replacements_json");
}

export interface RunningProgram {
  exe_name: string;
  display_name: string;
}

export function getRunningPrograms() {
  if (!inTauri()) return Promise.resolve([] as RunningProgram[]);
  return invoke<RunningProgram[]>("get_running_programs");
}

export function pickProgramFile() {
  if (!inTauri()) return Promise.resolve(null as string | null);
  return invoke<string | null>("pick_program_file");
}
