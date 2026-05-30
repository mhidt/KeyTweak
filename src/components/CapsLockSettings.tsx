import { useState } from "react";
import type {
  Config,
  RealCapsCombo,
  SwitchMethod,
  SwitchMode,
} from "../types/config";
import { capturedKeyId, normalizeKeyInput } from "../lib/keys";
import { KeyCaptureInput } from "./KeyCaptureInput";
import { Label } from "./ui/label";
import { Select } from "./ui/select";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

export function CapsLockSettings({ config, onChange }: Props) {
  const [capture, setCapture] = useState(false);

  const updateCaps = (patch: Partial<Config["caps_lock"]>) =>
    onChange({ ...config, caps_lock: { ...config.caps_lock, ...patch } });

  const handleCaptureKeyDown = (
    event: React.KeyboardEvent<HTMLButtonElement>,
  ) => {
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      setCapture(false);
      return;
    }

    const key = capturedKeyId(event);
    if (!key) return;

    updateCaps({ switch_key: key });
    setCapture(false);
  };

  const isCapsLock = config.caps_lock.switch_key === "capslock";

  return (
    <>
      <div className="space-y-2">
        <Label htmlFor="switch-key">Клавиша переключения</Label>
        <KeyCaptureInput
          id="switch-key"
          value={config.caps_lock.switch_key}
          onChange={(value) =>
            updateCaps({ switch_key: normalizeKeyInput(value) })
          }
          placeholder="capslock"
          active={capture}
          onInputFocus={() => setCapture(false)}
          onButtonFocus={() => setCapture(true)}
          onButtonBlur={() => setCapture(false)}
          onCaptureKeyDown={handleCaptureKeyDown}
          activeTitle="Нажмите клавишу"
          inactiveTitle="Считать клавишу с клавиатуры"
          wrapperClassName="w-[220px]"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="switch-method">Способ переключения</Label>
        <Select
          id="switch-method"
          value={config.caps_lock.switch_method}
          onChange={(event) =>
            updateCaps({ switch_method: event.target.value as SwitchMethod })
          }
          className="w-[400px]"
        >
          <option value="hotkey">Эмуляция Shift + Alt (рекомендуется)</option>
          <option value="programmatic">Программная установка (для нескольких раскладок)</option>
        </Select>
      </div>

      {config.caps_lock.switch_method === "programmatic" ? (
        <div className="space-y-2">
          <Label>Режим переключения</Label>
          <div className="flex gap-2">
            {[
              ["previous", "Предыдущий"],
              ["default", "По умолчанию"],
            ].map(([value, label]) => (
              <button
                key={value}
                type="button"
                className={
                  config.caps_lock.switch_mode === value
                    ? "h-9 rounded-md bg-primary px-4 text-sm text-primary-foreground"
                    : "h-9 rounded-md border border-border px-4 text-sm hover:bg-muted"
                }
                onClick={() => updateCaps({ switch_mode: value as SwitchMode })}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      ) : null}

      {isCapsLock ? (
        <div className="space-y-2">
          <Label htmlFor="real-caps-combo">
            Комбинация для настоящего Caps Lock
          </Label>
          <Select
            id="real-caps-combo"
            value={config.caps_lock.real_caps_combo}
            onChange={(event) =>
              updateCaps({
                real_caps_combo: event.target.value as RealCapsCombo,
              })
            }
            className="w-[220px]"
          >
            <option value="shift_caps">Shift + Caps Lock</option>
            <option value="alt_caps">Alt + Caps Lock</option>
            <option value="ctrl_caps">Ctrl + Caps Lock</option>
          </Select>
        </div>
      ) : null}
    </>
  );
}
