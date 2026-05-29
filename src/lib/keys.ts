import type React from "react";

export function normalizeKeyInput(value: string) {
  return value.trim().toLowerCase().replace(/\s+/g, "_");
}

export function capturedKeyId(event: React.KeyboardEvent) {
  const code = event.code.toLowerCase();

  // Sided modifiers: "controlleft" → "left_control", "shiftright" → "right_shift"
  if (code.endsWith("left") || code.endsWith("right")) {
    const side = code.endsWith("left") ? "left" : "right";
    const base = code.slice(0, -side.length).replace("meta", "win");
    return `${side}_${base}`;
  }

  if (code.startsWith("key")) return code.slice(3);
  if (code.startsWith("digit")) return code.slice(5);
  if (code.startsWith("numpad")) return code;
  if (code.startsWith("f") && code.length <= 3) return code;

  // For everything else, use event.key directly (browser-native names)
  const key = event.key.toLowerCase();
  if (key === " ") return "space";
  if (event.key.length === 1) return key;
  return key || "";
}

/**
 * Converts a KeyboardEvent into a hotkey combo string like "ctrl+shift+c".
 * Collects active modifiers and the pressed non-modifier key.
 */
export function capturedHotkeyCombo(event: React.KeyboardEvent): string | null {
  const modifiers: [boolean, string][] = [
    [event.ctrlKey, "ctrl"],
    [event.shiftKey, "shift"],
    [event.altKey, "alt"],
    [event.metaKey, "win"],
  ];
  const parts = modifiers.filter(([held]) => held).map(([, name]) => name);

  if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) return null;

  const key = capturedKeyId(event);
  if (!key) return null;

  return [...parts, key].join("+");
}
