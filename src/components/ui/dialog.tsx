import * as React from "react";
import { X } from "lucide-react";
import { Button } from "./button";

interface DialogProps {
  open: boolean;
  title: string;
  children: React.ReactNode;
  onClose: () => void;
}

export function Dialog({ open, title, children, onClose }: DialogProps) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/30">
      <div className="w-[420px] rounded-lg border border-border bg-background p-5 shadow-xl">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-base font-semibold">{title}</h2>
          <Button variant="ghost" size="icon" onClick={onClose} aria-label="Закрыть">
            <X size={16} />
          </Button>
        </div>
        {children}
      </div>
    </div>
  );
}
