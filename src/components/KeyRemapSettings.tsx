import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import type { Config, KeyRemap } from "../types/config";
import { capturedKeyId, normalizeKeyInput } from "../lib/keys";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import { KeyCaptureInput } from "./KeyCaptureInput";
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
  ) => {
    const active = capture?.index === index && capture.field === field;

    return (
      <KeyCaptureInput
        value={mapping[field]}
        onChange={(value) => updateMapping(index, { [field]: normalizeKeyInput(value) })}
        placeholder="left_alt"
        active={active}
        onInputFocus={() => setCapture(null)}
        onButtonFocus={() => setCapture({ index, field })}
        onButtonBlur={() => setCapture(null)}
        onCaptureKeyDown={(event) => handleCaptureKeyDown(event, index, field)}
        activeTitle="Нажмите клавишу"
        inactiveTitle="Считать клавишу с клавиатуры"
        wrapperClassName="min-w-[180px]"
      />
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
                    {renderKeyInput(mapping, index, "from")}
                  </Td>
                  <Td>
                    {renderKeyInput(mapping, index, "to")}
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
