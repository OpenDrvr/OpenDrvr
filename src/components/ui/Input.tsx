import type { LucideIcon } from "lucide-react";
import { useId, type InputHTMLAttributes, type ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
  error?: string;
  leadingIcon?: LucideIcon;
  trailing?: ReactNode;
  containerClassName?: string;
}

export function Input({
  label,
  hint,
  error,
  leadingIcon: LeadingIcon,
  trailing,
  containerClassName,
  className,
  id,
  ...props
}: InputProps) {
  const generatedId = useId();
  const inputId = id ?? generatedId;
  const description = error ?? hint;
  const descriptionId = description ? `${inputId}-description` : undefined;

  return (
    <div className={cn("space-y-1.5", containerClassName)}>
      {label && (
        <label htmlFor={inputId} className="block text-xs font-medium text-slate-300">
          {label}
        </label>
      )}
      <div className="relative">
        {LeadingIcon && (
          <LeadingIcon
            className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500"
            aria-hidden="true"
          />
        )}
        <input
          id={inputId}
          aria-invalid={error ? true : undefined}
          aria-describedby={descriptionId}
          className={cn(
            "h-9 w-full rounded-md border border-white/10 bg-surface-950/60 px-3 text-sm text-slate-100 placeholder:text-slate-500 transition-colors focus:border-accent-500/60 focus:outline-none focus:ring-2 focus:ring-accent-500/20 disabled:cursor-not-allowed disabled:opacity-60",
            LeadingIcon && "pl-9",
            trailing ? "pr-11" : undefined,
            error && "border-red-400/40 focus:border-red-400/60 focus:ring-red-400/20",
            className,
          )}
          {...props}
        />
        {trailing ? (
          <div className="absolute right-1.5 top-1/2 flex -translate-y-1/2 items-center">
            {trailing}
          </div>
        ) : null}
      </div>
      {description && (
        <p id={descriptionId} className={cn("text-xs", error ? "text-red-300" : "text-slate-400")}>
          {description}
        </p>
      )}
    </div>
  );
}
