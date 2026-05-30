import { Download, Pencil, Plus, Trash2, Upload, X } from "lucide-react";
import { useState } from "react";
import type { Config, ExceptionProgram, Replacement } from "../types/config";
import { exportReplacementsJson, importReplacementsJson } from "../lib/commands";
import { ProgramCombobox } from "./ProgramCombobox";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import { Dialog } from "./ui/dialog";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Table, Td, Th } from "./ui/table";

interface Props {
  config: Config;
  onChange: (config: Config) => void;
}

interface EditorState {
  index: number | null;
  short: string;
  replacement: string;
  exclusions: ExceptionProgram[];
}

function exclusionLabel(item: ExceptionProgram): string {
  return (
    item.display_name ||
    item.program.replace(/^.*[\\/]/, "").replace(/\.exe$/i, "")
  );
}

export function AutoReplaceSettings({ config, onChange }: Props) {
  const [editor, setEditor] = useState<EditorState | null>(null);
  const [exclusionInput, setExclusionInput] = useState("");
  const [exclusionDisplay, setExclusionDisplay] = useState("");
  const auto = config.auto_replace;

  const updateAuto = (patch: Partial<Config["auto_replace"]>) =>
    onChange({ ...config, auto_replace: { ...auto, ...patch } });

  const openEditor = (state: EditorState) => {
    setEditor(state);
    setExclusionInput("");
    setExclusionDisplay("");
  };

  const closeEditor = () => {
    setEditor(null);
    setExclusionInput("");
    setExclusionDisplay("");
  };

  const addExclusion = () => {
    if (!editor) return;
    const value = exclusionInput.trim();
    if (!value) return;
    if (
      editor.exclusions.some(
        (item) => item.program.toLowerCase() === value.toLowerCase(),
      )
    ) {
      setExclusionInput("");
      setExclusionDisplay("");
      return;
    }
    const name = exclusionDisplay.trim();
    setEditor({
      ...editor,
      exclusions: [
        ...editor.exclusions,
        {
          program: value,
          display_name: name && name !== value ? name : undefined,
        },
      ],
    });
    setExclusionInput("");
    setExclusionDisplay("");
  };

  const removeExclusion = (index: number) => {
    if (!editor) return;
    setEditor({
      ...editor,
      exclusions: editor.exclusions.filter((_, i) => i !== index),
    });
  };

  const saveReplacement = () => {
    if (!editor || !editor.short.trim()) return;
    const next = [...auto.replacements];
    const value: Replacement = {
      short: editor.short.trim(),
      replacement: editor.replacement,
      ...(editor.exclusions.length > 0
        ? { exclusions: editor.exclusions }
        : {}),
    };
    if (editor.index === null) next.push(value);
    else next[editor.index] = value;
    updateAuto({ replacements: next });
    closeEditor();
  };

  const exportJson = () => {
    const json = JSON.stringify(auto.replacements, null, 2);
    exportReplacementsJson(json);
  };

  const importJson = async () => {
    const content = await importReplacementsJson();
    if (!content) return;
    try {
      const parsed = JSON.parse(content) as Replacement[];
      updateAuto({
        replacements: parsed.filter(
          (item) => item.short && typeof item.replacement === "string",
        ),
      });
    } catch {
      // invalid JSON — ignore silently
    }
  };

  return (
    <>
      <div className="space-y-3">
        <Label>Триггеры</Label>
        <div className="flex flex-wrap gap-x-6 gap-y-3">
          <Checkbox
            checked={auto.trigger_space}
            onCheckedChange={(trigger_space) => updateAuto({ trigger_space })}
            label="Пробел"
          />
          <Checkbox
            checked={auto.trigger_tab}
            onCheckedChange={(trigger_tab) => updateAuto({ trigger_tab })}
            label="Tab"
          />
          <Checkbox
            checked={auto.trigger_enter}
            onCheckedChange={(trigger_enter) => updateAuto({ trigger_enter })}
            label="Enter"
          />
          <Checkbox
            checked={auto.trigger_punctuation}
            onCheckedChange={(trigger_punctuation) =>
              updateAuto({ trigger_punctuation })
            }
            label="Пунктуация (. , ? ! ; :)"
          />
        </div>
      </div>

      <div className="flex flex-wrap gap-x-6 gap-y-3">
        <Checkbox
          checked={auto.whole_words_only}
          onCheckedChange={(whole_words_only) =>
            updateAuto({ whole_words_only })
          }
          label="Только целые слова"
        />
        <Checkbox
          checked={auto.case_sensitive}
          onCheckedChange={(case_sensitive) => updateAuto({ case_sensitive })}
          label="С учётом регистра"
        />
      </div>

      <div className="flex gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={importJson}
        >
          <Download size={14} /> Импорт JSON
        </Button>
        <Button variant="outline" size="sm" onClick={exportJson}>
          <Upload size={14} /> Экспорт JSON
        </Button>
      </div>

      <div>
        <div className="mb-3 flex items-center justify-between">
          <Label>Список замен</Label>
          <Button
            size="sm"
            onClick={() =>
              openEditor({
                index: null,
                short: "",
                replacement: "",
                exclusions: [],
              })
            }
          >
            <Plus size={14} /> Добавить
          </Button>
        </div>
        <div className="overflow-hidden rounded-md border border-border">
          <Table>
            <thead>
              <tr>
                <Th>Шаблон</Th>
                <Th>Замена</Th>
                <Th>Исключения</Th>
                <Th className="w-24" />
              </tr>
            </thead>
            <tbody>
              {auto.replacements.map((entry, index) => (
                <tr key={`${entry.short}-${index}`}>
                  <Td>{entry.short}</Td>
                  <Td className="max-w-[280px] truncate">
                    {entry.replacement}
                  </Td>
                  <Td
                    className="max-w-[200px] truncate text-muted-foreground"
                    title={(entry.exclusions ?? [])
                      .map(exclusionLabel)
                      .join(", ")}
                  >
                    {entry.exclusions && entry.exclusions.length > 0
                      ? entry.exclusions.map(exclusionLabel).join(", ")
                      : "—"}
                  </Td>
                  <Td>
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() =>
                          openEditor({
                            index,
                            short: entry.short,
                            replacement: entry.replacement,
                            exclusions: entry.exclusions ?? [],
                          })
                        }
                        aria-label="Edit"
                      >
                        <Pencil size={14} />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() =>
                          updateAuto({
                            replacements: auto.replacements.filter(
                              (_, itemIndex) => itemIndex !== index,
                            ),
                          })
                        }
                        aria-label="Delete"
                      >
                        <Trash2 size={14} />
                      </Button>
                    </div>
                  </Td>
                </tr>
              ))}
              {auto.replacements.length === 0 ? (
                <tr>
                  <Td colSpan={4} className="text-muted-foreground">
                    Нет настроенных замен.
                  </Td>
                </tr>
              ) : null}
            </tbody>
          </Table>
        </div>
      </div>

      <Dialog
        open={editor !== null}
        title={
          editor?.index === null ? "Добавить замену" : "Редактировать замену"
        }
        onClose={closeEditor}
      >
        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="short">Шаблон</Label>
            <Input
              id="short"
              value={editor?.short ?? ""}
              onChange={(event) =>
                setEditor(
                  (current) =>
                    current && { ...current, short: event.target.value },
                )
              }
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="replacement">Замена</Label>
            <Input
              id="replacement"
              value={editor?.replacement ?? ""}
              onChange={(event) =>
                setEditor(
                  (current) =>
                    current && { ...current, replacement: event.target.value },
                )
              }
            />
          </div>
          <div className="space-y-2">
            <Label>Исключения (не работает в этих программах)</Label>
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <ProgramCombobox
                  value={exclusionInput}
                  displayValue={exclusionDisplay}
                  onChange={(v) => {
                    setExclusionInput(v);
                    setExclusionDisplay(v);
                  }}
                  onSubmit={addExclusion}
                  onSelectProgram={(exeName, displayName) => {
                    setExclusionInput(exeName);
                    setExclusionDisplay(displayName || exeName);
                  }}
                />
              </div>
              <Button type="button" onClick={addExclusion}>
                <Plus size={14} /> Добавить
              </Button>
            </div>
            {editor && editor.exclusions.length > 0 ? (
              <div className="flex flex-wrap gap-2 pt-1">
                {editor.exclusions.map((item, i) => (
                  <span
                    key={`${item.program}-${i}`}
                    className="inline-flex items-center gap-1 rounded-md border border-border bg-muted px-2 py-1 text-xs"
                    title={item.program}
                  >
                    {exclusionLabel(item)}
                    <button
                      type="button"
                      onClick={() => removeExclusion(i)}
                      className="text-muted-foreground hover:text-foreground"
                      aria-label="Удалить исключение"
                    >
                      <X size={12} />
                    </button>
                  </span>
                ))}
              </div>
            ) : (
              <p className="text-xs text-muted-foreground">
                Нет исключений — замена работает во всех программах.
              </p>
            )}
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={closeEditor}>
              Отмена
            </Button>
            <Button onClick={saveReplacement}>Сохранить</Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
