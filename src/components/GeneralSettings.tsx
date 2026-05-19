import type { Config } from "../types/config";
import { Checkbox } from "./ui/checkbox";
import { Label } from "./ui/label";
import { Select } from "./ui/select";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

export function GeneralSettings({ config, onChange }: Props) {
  return (
    <>
      <Checkbox
        checked={config.caps_lock.auto_start}
        onCheckedChange={(auto_start) => onChange({ ...config, caps_lock: { ...config.caps_lock, auto_start } })}
        label="Запускать вместе с Windows"
      />

      <div className="space-y-2">
        <Label htmlFor="app-language">Язык приложения</Label>
        <Select
          id="app-language"
          value={config.general.app_language}
          onChange={(event) => onChange({ ...config, general: { ...config.general, app_language: event.target.value } })}
          className="min-w-[180px]"
        >
          <option value="en">English</option>
          <option value="ru">Русский</option>
        </Select>
      </div>
    </>
  );
}

