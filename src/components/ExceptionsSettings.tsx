import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import type { Config, ExceptionMode } from "../types/config";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Select } from "./ui/select";
import { Table, Td, Th } from "./ui/table";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

export function ExceptionsSettings({ config, onChange }: Props) {
  const [program, setProgram] = useState("");
  const auto = config.auto_replace;

  const updateAutoReplace = (patch: Partial<Config["auto_replace"]>) =>
    onChange({ ...config, auto_replace: { ...auto, ...patch } });

  const updateExceptions = (exceptions: Config["auto_replace"]["exceptions"]) =>
    updateAutoReplace({ exceptions });

  const addException = () => {
    const value = program.trim();
    if (!value) return;
    updateExceptions([...auto.exceptions, { program: value }]);
    setProgram("");
  };

  return (
    <>
      <div className="max-w-[620px] space-y-1">
        <Label htmlFor="exception-mode">Режим списка</Label>
        <Select
          id="exception-mode"
          value={auto.exception_mode}
          onChange={(event) =>
            updateAutoReplace({ exception_mode: event.target.value as ExceptionMode })
          }
          className="w-[400px]"
        >
          <option value="blacklist">Чёрный список (работает везде, кроме этих программ)</option>
          <option value="whitelist">Белый список (работает только в этих программах)</option>
        </Select>
      </div>

      <div className="grid max-w-[620px] grid-cols-[1fr_auto] items-end gap-2">
        <div className="space-y-2">
          <Label htmlFor="program-name">Программа</Label>
          <Input
            id="program-name"
            placeholder="code.exe"
            value={program}
            onChange={(event) => setProgram(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") addException();
            }}
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
            {auto.exceptions.map((entry, index) => (
              <tr key={`${entry.program}-${index}`}>
                <Td>{entry.program}</Td>
                <Td className="text-muted-foreground">Все</Td>
                <Td>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() =>
                      updateExceptions(auto.exceptions.filter((_, i) => i !== index))
                    }
                    aria-label="Удалить исключение"
                  >
                    <Trash2 size={14} />
                  </Button>
                </Td>
              </tr>
            ))}
            {auto.exceptions.length === 0 ? (
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
