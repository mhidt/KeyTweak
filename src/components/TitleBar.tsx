import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, X } from "lucide-react";

const appWindow = getCurrentWindow();

export function TitleBar() {
  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.buttons === 1) {
      e.preventDefault();
      appWindow.startDragging();
    }
  };

  return (
    <div
      onMouseDown={handleMouseDown}
      className="flex h-9 shrink-0 select-none items-center border-b border-border bg-muted/55"
    >
      <div className="flex flex-1 items-center gap-2 px-3 pointer-events-none">
        <img
          src="/icon.ico"
          alt=""
          className="titlebar-icon h-4 w-4"
          draggable={false}
        />
        <span className="text-sm font-medium text-foreground">KeyTweak</span>
      </div>

      <div className="flex">
        <button
          type="button"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() => appWindow.minimize()}
          className="flex h-9 w-11 items-center justify-center text-foreground transition-colors hover:bg-muted"
          aria-label="Свернуть"
        >
          <Minus size={14} />
        </button>
        <button
          type="button"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() => appWindow.hide()}
          className="flex h-9 w-11 items-center justify-center text-foreground transition-colors hover:bg-red-500 hover:text-white"
          aria-label="Закрыть"
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
