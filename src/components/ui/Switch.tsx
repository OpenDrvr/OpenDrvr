import { useId } from "react";
import { cn } from "../../lib/cn";

export interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
  id?: string;
}

export function Switch({
  checked,
  onChange,
  label,
  description,
  disabled = false,
  id,
}: SwitchProps) {
  const generatedId = useId();
  const switchId = id ?? generatedId;
  const descriptionId = description ? `${switchId}-description` : undefined;

  return (
    <div className="flex items-start justify-between gap-6">
      <div className="min-w-0">
        <label htmlFor={switchId} className="text-xs font-medium text-slate-300">
          {label}
        </label>
        {description && (
          <p id={descriptionId} className="mt-0.5 text-xs leading-5 text-slate-400">
            {description}
          </p>
        )}
      </div>
      <button
        id={switchId}
        type="button"
        role="switch"
        aria-checked={checked}
        aria-describedby={descriptionId}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative mt-0.5 inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-400 disabled:cursor-not-allowed disabled:opacity-50",
          checked ? "border-accent-400/40 bg-accent-500" : "border-white/10 bg-white/[0.06]",
        )}
      >
        <span
          className={cn(
            "size-3.5 rounded-full bg-white shadow-sm transition-transform",
            checked ? "translate-x-[1.15rem]" : "translate-x-[0.15rem]",
          )}
          aria-hidden="true"
        />
      </button>
    </div>
  );
}
