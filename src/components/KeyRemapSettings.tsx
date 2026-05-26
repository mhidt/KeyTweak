import { Keyboard, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import type { Config, KeyRemap } from "../types/config";
import { cn } from "../lib/utils";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import { Label } from "./ui/label";
import { Table, Td, Th } from "./ui/table";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

type CaptureTarget = {
  index: number;
  field: "from" | "to";
};

const KEY_LABELS: Record<string, string> = {
  alt: "Alt",
  backspace: "Backspace",
  caps_lock: "Caps Lock",
  ctrl: "Ctrl",
  delete: "Delete",
  down: "Down",
  end: "End",
  enter: "Enter",
  esc: "Esc",
  home: "Home",
  insert: "Insert",
  left: "Left Arrow",
  left_alt: "Left Alt",
  left_ctrl: "Left Ctrl",
  left_shift: "Left Shift",
  left_win: "Left Win",
  page_down: "Page Down",
  page_up: "Page Up",
  right: "Right Arrow",
  right_alt: "Right Alt",
  right_ctrl: "Right Ctrl",
  right_shift: "Right Shift",
  right_win: "Right Win",
  shift: "Shift",
  space: "Space",
  tab: "Tab",
  up: "Up Arrow",
  win: "Win",
};

function normalizeKeyInput(value: string) {
  return value.trim().toLowerCase().replace(/\s+/g, "_");
}

function keyLabel(value: string) {
  const normalized = normalizeKeyInput(value);
  return KEY_LABELS[normalized] ?? normalized.toUpperCase();
}

function capturedKeyId(event: React.KeyboardEvent) {
  switch (event.code) {
    case "AltLeft":
      return "left_alt";
    case "AltRight":
      return "right_alt";
    case "ControlLeft":
      return "left_ctrl";
    case "ControlRight":
      return "right_ctrl";
    case "ShiftLeft":
      return "left_shift";
    case "ShiftRight":
      return "right_shift";
    case "MetaLeft":
      return "left_win";
    case "MetaRight":
      return "right_win";
    default:
      break;
  }

  if (/^Key[A-Z]$/.test(event.code)) return event.code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(event.code)) return event.code.slice(5);
  if (/^Numpad[0-9]$/.test(event.code)) return `num${event.code.slice(6)}`;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(event.code)) {
    return event.code.toLowerCase();
  }

  switch (event.key) {
    case " ":
    case "Spacebar":
      return "space";
    case "Escape":
      return "esc";
    case "ArrowUp":
      return "up";
    case "ArrowDown":
      return "down";
    case "ArrowLeft":
      return "left";
    case "ArrowRight":
      return "right";
    case "PageUp":
      return "page_up";
    case "PageDown":
      return "page_down";
    case "CapsLock":
      return "caps_lock";
    case "Backspace":
    case "Delete":
    case "End":
    case "Enter":
    case "Home":
    case "Insert":
    case "Tab":
      return event.key.toLowerCase();
    default:
      return event.key.length === 1 ? event.key.toLowerCase() : "";
  }
}

export function KeyRemapSettings({ config, onChange }: Props) {
  const [capture, setCapture] = useState<CaptureTarget | null>(null);
  const remap = config.key_remap;
  const updateRemap = (patch: Partial<Config["key_remap"]>) =>
    onChange({ ...config, key_remap: { ...remap, ...patch } });

  const updateMapping = (index: number, patch: Partial<KeyRemap>) => {
    updateRemap({
      mappings: remap.mappings.map((mapping, itemIndex) =>
        itemIndex === index ? { ...mapping, ...patch } : mapping,
      ),
    });
  };

  const addMapping = () => {
    updateRemap({
      mappings: [
        ...remap.mappings,
        { from: "left_alt", to: "win", enabled: false },
      ],
    });
  };

  const removeMapping = (index: number) => {
    updateRemap({
      mappings: remap.mappings.filter((_, itemIndex) => itemIndex !== index),
    });
  };

  const handleCaptureKeyDown = (
    event: React.KeyboardEvent<HTMLButtonElement>,
    index: number,
    field: "from" | "to",
  ) => {
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      setCapture(null);
      return;
    }

    const key = capturedKeyId(event);
    if (!key) return;

    updateMapping(index, { [field]: key });
    setCapture(null);
  };

  const renderKeyInput = (
    mapping: KeyRemap,
    index: number,
    field: "from" | "to",
    ariaLabel: string,
  ) => {
    const active = capture?.index === index && capture.field === field;
    const value = mapping[field];

    return (
      <div
        className={cn(
          "relative min-w-[180px]",
          active && "rounded-md ring-2 ring-primary/30",
        )}
      >
        <input
          value={value}
          onChange={(event) =>
            updateMapping(index, {
              [field]: normalizeKeyInput(event.target.value),
            })
          }
          onFocus={() => setCapture(null)}
          placeholder="left_alt"
          title={value ? keyLabel(value) : undefined}
          className="h-9 w-full rounded-md border border-input bg-background py-2 pl-3 pr-10 text-sm outline-none focus:ring-2 focus:ring-primary/30"
          aria-label={ariaLabel}
        />
        <button
          type="button"
          title={active ? "Нажмите клавишу" : "Считать клавишу с клавиатуры"}
          onFocus={() => setCapture({ index, field })}
          onBlur={() => setCapture(null)}
          onKeyDown={(event) => handleCaptureKeyDown(event, index, field)}
          className={cn(
            "absolute right-1 top-1 flex h-7 w-7 items-center justify-center rounded text-muted-foreground",
            active ? "bg-muted text-foreground" : "hover:bg-muted hover:text-foreground",
          )}
          aria-label="Считать клавишу с клавиатуры"
        >
          <Keyboard size={14} />
        </button>
      </div>
    );
  };

  return (
    <>
      <Checkbox
        checked={remap.enabled}
        onCheckedChange={(enabled) => updateRemap({ enabled })}
        label="Включить переназначение клавиш"
      />

      <div>
        <div className="mb-3 flex items-center justify-between">
          <Label>Таблица переназначений</Label>
          <Button size="sm" onClick={addMapping}>
            <Plus size={14} /> Добавить
          </Button>
        </div>

        <div className="overflow-hidden rounded-md border border-border">
          <Table>
            <thead>
              <tr>
                <Th className="w-28">Активно</Th>
                <Th>Клавиша</Th>
                <Th>Работает как</Th>
                <Th className="w-16" />
              </tr>
            </thead>
            <tbody>
              {remap.mappings.map((mapping, index) => (
                <tr key={index}>
                  <Td>
                    <input
                      type="checkbox"
                      checked={mapping.enabled}
                      onChange={(event) =>
                        updateMapping(index, { enabled: event.target.checked })
                      }
                      className="h-4 w-4 accent-zinc-900"
                      aria-label="Активно"
                    />
                  </Td>
                  <Td>
                    {renderKeyInput(mapping, index, "from", "Клавиша")}
                  </Td>
                  <Td>
                    {renderKeyInput(mapping, index, "to", "Работает как")}
                  </Td>
                  <Td>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => removeMapping(index)}
                      aria-label="Удалить"
                    >
                      <Trash2 size={14} />
                    </Button>
                  </Td>
                </tr>
              ))}
              {remap.mappings.length === 0 ? (
                <tr>
                  <Td colSpan={4} className="text-muted-foreground">
                    Нет переназначений.
                  </Td>
                </tr>
              ) : null}
            </tbody>
          </Table>
        </div>
      </div>
    </>
  );
}
