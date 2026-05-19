import { Eye, EyeOff } from "lucide-react";
import { useState } from "react";
import { testTranslateApi } from "../lib/commands";
import type { Config } from "../types/config";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Select } from "./ui/select";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

export function TranslateSettings({ config, onChange }: Props) {
  const [showKey, setShowKey] = useState(false);
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState("");
  const translate = config.translate;

  const updateTranslate = (patch: Partial<Config["translate"]>) =>
    onChange({ ...config, translate: { ...translate, ...patch } });

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

  return (
    <>
      <div className="space-y-2">
        <Label htmlFor="translate-server-url">Адрес сервера LibreTranslate</Label>
        <Input
          id="translate-server-url"
          className="max-w-[520px]"
          placeholder="http://127.0.0.1:5000"
          value={translate.server_url}
          onChange={(event) => updateTranslate({ server_url: event.target.value })}
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
            onChange={(event) => updateTranslate({ api_key: event.target.value })}
          />
          <Button variant="outline" size="icon" onClick={() => setShowKey((value) => !value)}           aria-label="Показать/скрыть API-ключ">
            {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          Нужен только для серверов с авторизацией (например, libretranslate.com).
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="target-language">Целевой язык</Label>
        <Select
          id="target-language"
          value={translate.target_language}
          onChange={(event) => updateTranslate({ target_language: event.target.value })}
        >
          <option value="ru">RU</option>
          <option value="en">EN</option>
        </Select>
      </div>

      <div className="grid max-w-[520px] grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label htmlFor="translate-hotkey">Горячая клавиша перевода</Label>
          <Input
            id="translate-hotkey"
            value={translate.hotkey_translate}
            onChange={(event) => updateTranslate({ hotkey_translate: event.target.value })}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="reverse-hotkey">Горячая клавиша обратного перевода</Label>
          <Input
            id="reverse-hotkey"
            value={translate.hotkey_reverse}
            onChange={(event) => updateTranslate({ hotkey_reverse: event.target.value })}
          />
        </div>
      </div>

      <div className="flex items-center gap-3">
        <Button variant="outline" onClick={checkApi} disabled={checking || !translate.server_url.trim()}>
          {checking ? "Проверка..." : "Проверить связь"}
        </Button>
        {result ? <span className="text-sm text-muted-foreground">{result}</span> : null}
      </div>
    </>
  );
}
