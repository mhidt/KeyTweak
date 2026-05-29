import { Eye, EyeOff } from "lucide-react";
import { useRef, useState } from "react";
import { testTranslateApi } from "../lib/commands";
import { capturedHotkeyCombo } from "../lib/keys";
import type { Config } from "../types/config";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import { Input } from "./ui/input";
import { KeyCaptureInput } from "./KeyCaptureInput";
import { Label } from "./ui/label";
import { Select } from "./ui/select";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

type HotkeyCaptureField = "hotkey_translate" | "hotkey_reverse";

export function TranslateSettings({ config, onChange }: Props) {
  const [showKey, setShowKey] = useState(false);
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState("");
  const [captureField, setCaptureField] = useState<HotkeyCaptureField | null>(null);
  const [captureBuffer, setCaptureBuffer] = useState<string[]>([]);
  const commitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const translate = config.translate;

  const updateTranslate = (patch: Partial<Config["translate"]>) =>
    onChange({ ...config, translate: { ...translate, ...patch } });

  const commitCapture = (field: HotkeyCaptureField, parts: string[]) => {
    if (parts.length > 0) {
      updateTranslate({ [field]: parts.join("+") });
    }
    setCaptureBuffer([]);
    setCaptureField(null);
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
      commitTimerRef.current = null;
    }
  };

  const cancelCapture = () => commitCapture(captureField!, []);

  const handleHotkeyCapture = (
    event: React.KeyboardEvent<HTMLButtonElement>,
    field: HotkeyCaptureField,
  ) => {
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      cancelCapture();
      return;
    }

    if (event.key === "Enter") {
      commitCapture(field, captureBuffer);
      return;
    }

    const combo = capturedHotkeyCombo(event);
    if (!combo) return;

    // Parse the combo into parts (e.g. "ctrl+c" -> ["ctrl", "c"])
    const comboParts = combo.split("+");

    // Build updated buffer: replace modifiers from previous step, append new key
    // Strategy: keep accumulating. Each keydown adds its full combo representation.
    // For "ctrl+c+c": first press gives ["ctrl","c"], second press appends "c".
    const newBuffer = [...captureBuffer];

    if (newBuffer.length === 0) {
      // First press: take all parts
      newBuffer.push(...comboParts);
    } else {
      // Subsequent presses: only add the non-modifier key part
      // (modifiers are already captured from first press)
      const mainKey = comboParts[comboParts.length - 1];
      newBuffer.push(mainKey);
    }

    setCaptureBuffer(newBuffer);
    updateTranslate({ [field]: newBuffer.join("+") });

    // Reset the auto-commit timer
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
    }
    commitTimerRef.current = setTimeout(() => {
      commitCapture(field, newBuffer);
    }, 1500);
  };

  const handleCaptureBlur = (field: HotkeyCaptureField) => {
    if (captureBuffer.length > 0) {
      commitCapture(field, captureBuffer);
    } else {
      cancelCapture();
    }
  };

  const checkApi = async () => {
    setChecking(true);
    setResult("");
    try {
      const translated = await testTranslateApi(
        translate.server_url,
        translate.api_key,
        translate.target_language,
      );
      setResult(`Сервер доступен. "hello" → "${translated}"`);
    } catch (error) {
      setResult(error instanceof Error ? error.message : String(error));
    } finally {
      setChecking(false);
    }
  };

  const renderHotkeyInput = (
    id: string,
    label: string,
    field: HotkeyCaptureField,
    wrapperClassName?: string,
  ) => (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <KeyCaptureInput
        id={id}
        value={translate[field]}
        onChange={(value) => updateTranslate({ [field]: value })}
        active={captureField === field}
        onInputFocus={() => cancelCapture()}
        onButtonFocus={() => { setCaptureField(field); setCaptureBuffer([]); }}
        onButtonBlur={() => handleCaptureBlur(field)}
        onCaptureKeyDown={(event) => handleHotkeyCapture(event, field)}
        wrapperClassName={wrapperClassName}
      />
    </div>
  );

  return (
    <>
      <div className="space-y-2">
        <Label htmlFor="translate-server-url">
          Адрес сервера LibreTranslate
        </Label>
        <Input
          id="translate-server-url"
          className="max-w-[520px]"
          placeholder="http://127.0.0.1:5000"
          value={translate.server_url}
          onChange={(event) =>
            updateTranslate({ server_url: event.target.value })
          }
        />
        <p className="text-xs text-muted-foreground">
          Адрес вашего экземпляра LibreTranslate (локальный или удалённый).
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="translate-api-key">API-ключ (необязательно)</Label>
        <div className="flex max-w-[520px] gap-2">
          <Input
            id="translate-api-key"
            type={showKey ? "text" : "password"}
            placeholder="Оставьте пустым для локальных серверов"
            value={translate.api_key}
            onChange={(event) =>
              updateTranslate({ api_key: event.target.value })
            }
          />
          <Button
            variant="outline"
            size="icon"
            onClick={() => setShowKey((value) => !value)}
            aria-label="Показать/скрыть API-ключ"
          >
            {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          Нужен только для серверов с авторизацией (например,
          libretranslate.com).
        </p>
      </div>

      <div className="max-w-[520px] space-y-4">
        {renderHotkeyInput("translate-hotkey", "Горячая клавиша перевода", "hotkey_translate", "max-w-[280px]")}

        <div className="rounded-md border border-border bg-muted/35 px-4 py-3">
          <Checkbox
            checked={translate.auto_detect_language}
            onCheckedChange={(auto_detect_language) =>
              updateTranslate({ auto_detect_language })
            }
            label="Автоопределение языка"
            description="Русский текст переводится на EN, остальной текст — на RU."
          />
        </div>

        {!translate.auto_detect_language ? (
          <div className="grid grid-cols-[160px_1fr] gap-4 rounded-md border border-border bg-background p-4">
            <div className="space-y-2">
              <Label htmlFor="target-language">Целевой язык</Label>
              <Select
                id="target-language"
                value={translate.target_language}
                onChange={(event) =>
                  updateTranslate({ target_language: event.target.value })
                }
                className="w-full"
              >
                <option value="ru">RU</option>
                <option value="en">EN</option>
              </Select>
            </div>
            {renderHotkeyInput("reverse-hotkey", "Горячая клавиша обратного перевода", "hotkey_reverse")}
          </div>
        ) : null}
      </div>

      <div className="flex items-center gap-3">
        <Button
          variant="outline"
          onClick={checkApi}
          disabled={checking || !translate.server_url.trim()}
        >
          {checking ? "Проверка..." : "Проверить связь"}
        </Button>
        {result ? (
          <span className="text-sm text-muted-foreground">{result}</span>
        ) : null}
      </div>
    </>
  );
}
