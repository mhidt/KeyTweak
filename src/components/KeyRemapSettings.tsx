import { Plus, Trash2 } from "lucide-react";
import type { Config, KeyRemap } from "../types/config";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import { Label } from "./ui/label";
import { Select } from "./ui/select";
import { Table, Td, Th } from "./ui/table";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

const KEY_OPTIONS = [
  { value: "alt", label: "Alt" },
  { value: "win", label: "Win" },
  { value: "ctrl", label: "Ctrl" },
  { value: "shift", label: "Shift" },
  { value: "caps_lock", label: "Caps Lock" },
  { value: "tab", label: "Tab" },
  { value: "esc", label: "Esc" },
  { value: "enter", label: "Enter" },
  { value: "backspace", label: "Backspace" },
  { value: "space", label: "Space" },
];

export function KeyRemapSettings({ config, onChange }: Props) {
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
      mappings: [...remap.mappings, { from: "alt", to: "win", enabled: true }],
    });
  };

  const removeMapping = (index: number) => {
    updateRemap({
      mappings: remap.mappings.filter((_, itemIndex) => itemIndex !== index),
    });
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
                <tr key={`${mapping.from}-${mapping.to}-${index}`}>
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
                    <Select
                      value={mapping.from}
                      onChange={(event) =>
                        updateMapping(index, { from: event.target.value })
                      }
                      className="w-full min-w-[140px]"
                    >
                      {KEY_OPTIONS.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </Select>
                  </Td>
                  <Td>
                    <Select
                      value={mapping.to}
                      onChange={(event) =>
                        updateMapping(index, { to: event.target.value })
                      }
                      className="w-full min-w-[140px]"
                    >
                      {KEY_OPTIONS.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </Select>
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
