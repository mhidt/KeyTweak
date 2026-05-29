import {
  Ban,
  FileText,
  Globe,
  Keyboard,
  Replace,
  Zap,
} from "lucide-react";
import { cn } from "../lib/utils";

export type TabId =
  | "caps"
  | "autoreplace"
  | "keyremap"
  | "translate"
  | "exceptions"
  | "general";

const items = [
  { id: "caps", label: "Caps Lock", icon: Keyboard, group: "Модули" },
  { id: "autoreplace", label: "Автозамена", icon: FileText, group: "Модули" },
  { id: "keyremap", label: "Клавиши", icon: Replace, group: "Модули" },
  { id: "translate", label: "Перевод", icon: Globe, group: "Модули" },
  { id: "exceptions", label: "Исключения", icon: Ban, group: "Система" },
  { id: "general", label: "Общие", icon: Zap, group: "Система" },
] satisfies Array<{
  id: TabId;
  label: string;
  icon: typeof Keyboard;
  group: string;
}>;

interface SidebarProps {
  activeTab: TabId;
  onChange: (tab: TabId) => void;
}

export function Sidebar({ activeTab, onChange }: SidebarProps) {
  let currentGroup = "";

  return (
    <aside className="w-[200px] shrink-0 border-r border-border bg-muted/55 py-4">
      {items.map((item) => {
        const Icon = item.icon;
        const showGroup = item.group !== currentGroup;
        currentGroup = item.group;

        return (
          <div key={item.id}>
            {showGroup ? (
              <div
                className={cn(
                  "px-4 pb-2 pt-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground first:pt-0",
                  item.group === "Система" && "mt-4",
                )}
              >
                {item.group}
              </div>
            ) : null}
            <button
              className={cn(
                "mx-2 flex h-9 w-[184px] items-center gap-2 rounded-md px-3 text-left text-sm transition-colors",
                activeTab === item.id
                  ? "bg-primary text-primary-foreground"
                  : "text-foreground hover:bg-background",
              )}
              onClick={() => onChange(item.id)}
              type="button"
            >
              <Icon size={16} />
              <span className="truncate">{item.label}</span>
            </button>
          </div>
        );
      })}
    </aside>
  );
}
