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
  const [mode, setMode] = useState<ExceptionMode>("blacklist");
  const auto = config.auto_replace;

  const updateExceptions = (exceptions: Config["auto_replace"]["exceptions"]) =>
    onChange({ ...config, auto_replace: { ...auto, exceptions } });

  const addException = () => {
    const value = program.trim();
    if (!value) return;
    updateExceptions([...auto.exceptions, { program: value, mode }]);
    setProgram("");
    setMode("blacklist");
  };

  return (
    <>
      <div className="grid max-w-[620px] grid-cols-[1fr_150px_auto] items-end gap-2">
        <div className="space-y-2">
          <Label htmlFor="program-name">Программа</Label>
          <Input id="program-name" placeholder="code.exe" value={program} onChange={(event) => setProgram(event.target.value)} />
        </div>
        <div className="space-y-2">
          <Label htmlFor="exception-mode">Режим</Label>
          <Select id="exception-mode" value={mode} onChange={(event) => setMode(event.target.value as ExceptionMode)} className="w-full">
            <option value="blacklist">Чёрный список</option>
            <option value="whitelist">Белый список</option>
          </Select>
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
              <Th>Режим</Th>
              <Th className="w-16" />
            </tr>
          </thead>
          <tbody>
            {auto.exceptions.map((entry, index) => (
              <tr key={`${entry.program}-${index}`}>
                <Td>{entry.program}</Td>
                 <Td className="capitalize">{entry.mode === "blacklist" ? "Чёрный список" : "Белый список"}</Td>
                <Td>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => updateExceptions(auto.exceptions.filter((_, itemIndex) => itemIndex !== index))}
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

