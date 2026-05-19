import type { ReactNode } from "react";
import { Button } from "./ui/button";

interface PanelShellProps {
  title: string;
  children: ReactNode;
  onSave: () => void;
  onCancel: () => void;
  status?: string;
  saving?: boolean;
}

export function PanelShell({ title, children, onSave, onCancel, status, saving }: PanelShellProps) {
  return (
    <section className="flex h-full flex-col">
      <div className="flex-1 overflow-auto px-8 py-6">
        <h1 className="mb-6 text-lg font-semibold">{title}</h1>
        <div className="space-y-6">{children}</div>
      </div>
      <footer className="flex min-h-16 items-center justify-between border-t border-border px-8">
        <div className="text-xs text-muted-foreground">{status}</div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={onCancel} disabled={saving}>
            Отмена
          </Button>
          <Button onClick={onSave} disabled={saving}>
            Сохранить
          </Button>
        </div>
      </footer>
    </section>
  );
}

