import { Eye, EyeOff } from "lucide-react";
import { useState } from "react";
import type { Config } from "../types/config";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

export function ApiKeysSettings({ config, onChange }: Props) {
  const [showKey, setShowKey] = useState(false);

  const updateTranslate = (patch: Partial<Config["translate"]>) =>
    onChange({ ...config, translate: { ...config.translate, ...patch } });

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label htmlFor="api-server-url">Адрес сервера LibreTranslate</Label>
        <Input
          id="api-server-url"
          className="max-w-[520px]"
          placeholder="http://127.0.0.1:5000"
          value={config.translate.server_url}
          onChange={(event) => updateTranslate({ server_url: event.target.value })}
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="api-key-system">API-ключ LibreTranslate (необязательно)</Label>
        <div className="flex max-w-[520px] gap-2">
          <Input
            id="api-key-system"
            type={showKey ? "text" : "password"}
            placeholder="Оставьте пустым для локальных серверов"
            value={config.translate.api_key}
            onChange={(event) => updateTranslate({ api_key: event.target.value })}
          />
          <Button variant="outline" size="icon" onClick={() => setShowKey((value) => !value)}           aria-label="Показать/скрыть API-ключ">
            {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
          </Button>
        </div>
      </div>
    </div>
  );
}
