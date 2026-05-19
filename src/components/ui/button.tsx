import * as React from "react";
import { cn } from "../../lib/utils";

type ButtonVariant = "default" | "outline" | "ghost" | "destructive";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: "default" | "sm" | "icon";
}

const variantClasses: Record<ButtonVariant, string> = {
  default: "bg-primary text-primary-foreground hover:bg-primary/90",
  outline: "border border-border bg-background hover:bg-muted",
  ghost: "hover:bg-muted",
  destructive: "bg-red-600 text-white hover:bg-red-700",
};

const sizeClasses = {
  default: "h-9 px-4 py-2",
  sm: "h-8 px-3 text-xs",
  icon: "h-8 w-8",
};

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "default", ...props }, ref) => (
    <button
      ref={ref}
      className={cn(
        "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50",
        variantClasses[variant],
        sizeClasses[size],
        className,
      )}
      {...props}
    />
  ),
);
Button.displayName = "Button";

