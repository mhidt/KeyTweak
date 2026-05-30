import { ChevronDown, Plus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Config, ExceptionMode, ModuleId } from "../types/config";
import { ProgramCombobox } from "./ProgramCombobox";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { Select } from "./ui/select";
import { Table, Td, Th } from "./ui/table";

const ALL_MODULES: { id: ModuleId; label: string }[] = [
  { id: "caps_lock", label: "Смена языка" },
  { id: "auto_replace", label: "Автозамена" },
  { id: "key_remap", label: "Клавиши" },
  { id: "translate", label: "Перевод" },
];

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

function ModuleSelect({
  value,
  onChange,
}: {
  value: ModuleId[] | undefined;
  onChange: (modules: ModuleId[] | undefined) => void;
}) {
  const [open, setOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ top: 0, left: 0 });

  const selected = value ?? ALL_MODULES.map((m) => m.id);
  const allSelected = selected.length === ALL_MODULES.length;

  const toggle = (id: ModuleId) => {
    const next = selected.includes(id)
      ? selected.filter((m) => m !== id)
      : [...selected, id];
    onChange(next.length === ALL_MODULES.length ? undefined : next);
  };

  const handleOpen = () => {
    if (!open && buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      setPos({ top: rect.bottom + 4, left: rect.left });
    }
    setOpen(!open);
  };

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handleMouseDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        !buttonRef.current?.contains(target) &&
        !dropdownRef.current?.contains(target)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handleMouseDown);
    return () => document.removeEventListener("mousedown", handleMouseDown);
  }, [open]);

  const missingCount = ALL_MODULES.length - selected.length;
  const label = allSelected
    ? "Все"
    : selected.length === 0
      ? "Нет"
      : missingCount === 1
        ? `Все, кроме ${ALL_MODULES.find((m) => !selected.includes(m.id))?.label}`
        : selected.map((id) => ALL_MODULES.find((m) => m.id === id)?.label).join(", ");

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        onClick={handleOpen}
        className="flex min-w-[160px] items-center justify-between gap-1 rounded-md border border-border bg-background px-2 py-1 text-left text-sm hover:bg-accent"
      >
        <span className="truncate">{label}</span>
        <ChevronDown size={14} className="shrink-0 text-muted-foreground" />
      </button>
      {open && (
        <div
          ref={dropdownRef}
          style={{ top: pos.top, left: pos.left }}
          className="fixed z-[9999] min-w-[160px] rounded-md border border-border bg-background p-1 shadow-md"
        >
          {ALL_MODULES.map((mod) => (
            <label
              key={mod.id}
              className="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-sm hover:bg-muted"
            >
              <input
                type="checkbox"
                checked={selected.includes(mod.id)}
                onChange={() => toggle(mod.id)}
                className="h-4 w-4 accent-zinc-900"
              />
              {mod.label}
            </label>
          ))}
        </div>
      )}
    </>
  );
}

export function ExceptionsSettings({ config, onChange }: Props) {
  const [program, setProgram] = useState("");
  const [displayValue, setDisplayValue] = useState("");

  const updateExceptions = (exceptions: Config["exceptions"]) =>
    onChange({ ...config, exceptions });

  const addException = () => {
    const value = program.trim();
    if (!value) return;
    const name = displayValue.trim();
    updateExceptions([...config.exceptions, { program: value, display_name: (name && name !== value) ? name : undefined }]);
    setProgram("");
    setDisplayValue("");
  };

  const handleSelectProgram = (exeName: string, displayName: string) => {
    setProgram(exeName);
    setDisplayValue(displayName || exeName);
  };

  const updateModules = (index: number, modules: ModuleId[] | undefined) => {
    const next = [...config.exceptions];
    next[index] = { ...next[index], modules };
    updateExceptions(next);
  };

  return (
    <>
      <div className="max-w-[620px] space-y-1">
        <Label htmlFor="exception-mode">Режим списка</Label>
        <Select
          id="exception-mode"
          value={config.exception_mode}
          onChange={(event) =>
            onChange({ ...config, exception_mode: event.target.value as ExceptionMode })
          }
          className="w-[450px]"
        >
          <option value="blacklist">Чёрный список (модули не работают в этих программах)</option>
          <option value="whitelist">Белый список (модули работают только в этих программах)</option>
        </Select>
      </div>

      <div className="grid max-w-[620px] grid-cols-[1fr_auto] items-end gap-2">
        <div className="space-y-2">
          <Label>Программа</Label>
          <ProgramCombobox
            value={program}
            displayValue={displayValue}
            onChange={(v) => { setProgram(v); setDisplayValue(v); }}
            onSubmit={addException}
            onSelectProgram={handleSelectProgram}
          />
        </div>
        <Button onClick={addException}>
          <Plus size={14} /> Добавить
        </Button>
      </div>

      <div className="overflow-hidden rounded-md border border-border">
        <Table>
          <thead>
            <tr>
              <Th>Программа</Th>
              <Th>Модули</Th>
              <Th className="w-16" />
            </tr>
          </thead>
          <tbody>
            {config.exceptions.map((entry, index) => (
              <tr key={`${entry.program}-${index}`}>
                <Td title={entry.program}>
                  {entry.display_name || entry.program.replace(/^.*[\\/]/, "").replace(/\.exe$/i, "")}
                </Td>
                <Td>
                  <div className="flex justify-start">
                    <ModuleSelect
                      value={entry.modules}
                      onChange={(modules) => updateModules(index, modules)}
                    />
                  </div>
                </Td>
                <Td>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() =>
                      updateExceptions(config.exceptions.filter((_, i) => i !== index))
                    }
                    aria-label="Удалить исключение"
                  >
                    <Trash2 size={14} />
                  </Button>
                </Td>
              </tr>
            ))}
            {config.exceptions.length === 0 ? (
              <tr>
                <Td colSpan={3} className="text-muted-foreground">
                  Нет настроенных исключений.
                </Td>
              </tr>
            ) : null}
          </tbody>
        </Table>
      </div>
    </>
  );
}
