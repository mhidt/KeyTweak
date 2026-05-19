import { cn } from "../../lib/utils";

interface CheckboxProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
  className?: string;
}

export function Checkbox({
  checked,
  onCheckedChange,
  label,
  description,
  disabled,
  className,
}: CheckboxProps) {
  return (
    <label className={cn("flex cursor-pointer items-start gap-2 text-sm", disabled && "opacity-60", className)}>
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onCheckedChange(event.target.checked)}
        className="mt-0.5 h-4 w-4 accent-zinc-900"
      />
      <span>
        <span className="block leading-5">{label}</span>
        {description ? <span className="block text-xs text-muted-foreground">{description}</span> : null}
      </span>
    </label>
  );
}

