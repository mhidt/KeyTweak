import { ChevronDown, FolderOpen } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { getRunningPrograms, pickProgramFile, type RunningProgram } from "../lib/commands";
import { Input } from "./ui/input";

interface Props {
  value: string;
  displayValue: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onSelectProgram: (exeName: string, displayName: string) => void;
  placeholder?: string;
}

export function ProgramCombobox({
  value,
  displayValue,
  onChange,
  onSubmit,
  onSelectProgram,
  placeholder = "code.exe",
}: Props) {
  const [open, setOpen] = useState(false);
  const [programs, setPrograms] = useState<RunningProgram[]>([]);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const fetchPrograms = async () => {
    setLoading(true);
    try {
      const result = await getRunningPrograms();
      setPrograms(result);
    } catch {
      setPrograms([]);
    } finally {
      setLoading(false);
    }
  };

  const handleFocus = () => {
    fetchPrograms();
    setOpen(true);
  };

  const handleSelect = (exeName: string, displayName: string) => {
    onSelectProgram(exeName, displayName);
    setOpen(false);
    inputRef.current?.focus();
  };

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handleMouseDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (!containerRef.current?.contains(target)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handleMouseDown);
    return () => document.removeEventListener("mousedown", handleMouseDown);
  }, [open]);

  const searchTerm = displayValue.toLowerCase();
  const filtered = programs.filter(
    (p) =>
      p.exe_name.toLowerCase().includes(searchTerm) ||
      p.display_name.toLowerCase().includes(searchTerm),
  );

  return (
    <div ref={containerRef} className="relative">
      <div className="flex items-center gap-0">
        <Input
          ref={inputRef}
          placeholder={placeholder}
          value={displayValue}
          title={value}
          onChange={(event) => {
            onChange(event.target.value);
            if (!open) setOpen(true);
          }}
          onFocus={handleFocus}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              setOpen(false);
              onSubmit();
            }
            if (event.key === "Escape") setOpen(false);
          }}
          className="rounded-r-none"
          autoComplete="off"
        />
        <button
          type="button"
          onClick={async () => {
            const selected = await pickProgramFile();
            if (selected) {
              onChange(selected);
              setOpen(false);
            }
          }}
          title="Выбрать файл программы"
          className="flex h-9 items-center border border-l-0 border-input bg-background px-2 hover:bg-muted"
        >
          <FolderOpen size={14} className="text-muted-foreground" />
        </button>
        <button
          type="button"
          onClick={() => {
            if (!open) fetchPrograms();
            setOpen(!open);
          }}
          className="flex h-9 items-center rounded-r-md border border-l-0 border-input bg-background px-2 hover:bg-muted"
        >
          <ChevronDown size={14} className="text-muted-foreground" />
        </button>
      </div>
      {open && (
        <div
          ref={dropdownRef}
          className="absolute left-0 top-full z-[9999] mt-1 max-h-[200px] w-full overflow-y-auto rounded-md border border-border bg-background shadow-md"
        >
          {loading ? (
            <div className="px-3 py-2 text-sm text-muted-foreground">
              Загрузка...
            </div>
          ) : filtered.length === 0 ? (
            <div className="px-3 py-2 text-sm text-muted-foreground">
              {value ? "Нет совпадений" : "Нет запущенных программ"}
            </div>
          ) : (
            filtered.map((p) => (
              <button
                key={p.exe_name}
                type="button"
                className="flex w-full flex-col items-start px-3 py-1.5 text-left text-sm hover:bg-muted"
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => handleSelect(p.exe_name, p.display_name)}
              >
                <span className="font-medium">{p.display_name || p.exe_name}</span>
                {p.display_name && (
                  <span className="truncate text-xs text-muted-foreground">
                    {p.exe_name}
                  </span>
                )}
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
