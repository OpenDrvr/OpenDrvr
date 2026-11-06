import { ChevronDown } from "lucide-react";
import { useId } from "react";
import { cn } from "../../lib/cn";

export interface DropdownOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface DropdownProps {
  value: string;
  onChange: (value: string) => void;
  options: readonly DropdownOption[];
  label?: string;
  ariaLabel?: string;
  hint?: string;
  disabled?: boolean;
  id?: string;
  className?: string;
}

export function Dropdown({
  value,
  onChange,
  options,
  label,
  ariaLabel,
  hint,
  disabled = false,
  id,
  className,
}: DropdownProps) {
  const generatedId = useId();
  const selectId = id ?? generatedId;
  const hintId = hint ? `${selectId}-hint` : undefined;

  return (
    <div className={cn("space-y-1.5", className)}>
      {label && (
        <label htmlFor={selectId} className="block text-xs font-medium text-slate-300">
          {label}
        </label>
      )}
      <div className="relative">
        <select
          id={selectId}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          disabled={disabled}
          aria-label={label ? undefined : ariaLabel}
          aria-describedby={hintId}
          className="h-9 w-full appearance-none rounded-md border border-white/10 bg-surface-950/60 pl-3 pr-9 text-sm text-slate-100 transition-colors focus:border-accent-500/60 focus:outline-none focus:ring-2 focus:ring-accent-500/20 disabled:cursor-not-allowed disabled:opacity-60"
        >
          {options.map((option) => (
            <option key={option.value} value={option.value} disabled={option.disabled}>
              {option.label}
            </option>
          ))}
        </select>
        <ChevronDown
          className="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-500"
          aria-hidden="true"
        />
      </div>
      {hint && (
        <p id={hintId} className="text-xs text-slate-400">
          {hint}
        </p>
      )}
    </div>
  );
}
