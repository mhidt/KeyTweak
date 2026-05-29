import { Keyboard } from "lucide-react";
import { cn } from "../lib/utils";

interface Props {
  id?: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  active: boolean;
  onInputFocus: () => void;
  onButtonFocus: () => void;
  onButtonBlur: () => void;
  onCaptureKeyDown: (event: React.KeyboardEvent<HTMLButtonElement>) => void;
  activeTitle?: string;
  inactiveTitle?: string;
  wrapperClassName?: string;
}

export function KeyCaptureInput({
  id,
  value,
  onChange,
  placeholder,
  active,
  onInputFocus,
  onButtonFocus,
  onButtonBlur,
  onCaptureKeyDown,
  activeTitle = "Нажимайте клавиши (Enter — сохранить, Esc — отмена)",
  inactiveTitle = "Считать с клавиатуры",
  wrapperClassName,
}: Props) {
  return (
    <div
      className={cn(
        "relative",
        wrapperClassName,
        active && "rounded-md ring-2 ring-primary/30",
      )}
    >
      <input
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onFocus={onInputFocus}
        placeholder={placeholder}
        className="h-9 w-full rounded-md border border-input bg-background py-2 pl-3 pr-10 text-sm outline-none focus:ring-2 focus:ring-primary/30"
      />
      <button
        type="button"
        title={active ? activeTitle : inactiveTitle}
        onFocus={onButtonFocus}
        onBlur={onButtonBlur}
        onKeyDown={onCaptureKeyDown}
        className={cn(
          "absolute right-1 top-1 flex h-7 w-7 items-center justify-center rounded text-muted-foreground",
          active ? "bg-muted text-foreground" : "hover:bg-muted hover:text-foreground",
        )}
        aria-label={inactiveTitle}
      >
        <Keyboard size={14} />
      </button>
    </div>
  );
}
