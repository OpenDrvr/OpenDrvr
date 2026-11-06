import { Search, X } from "lucide-react";
import { cn } from "../../lib/cn";

export interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  label?: string;
  disabled?: boolean;
  shortcut?: string;
  className?: string;
}

export function SearchField({
  value,
  onChange,
  placeholder = "Search…",
  label = "Search",
  disabled = false,
  shortcut,
  className,
}: SearchFieldProps) {
  return (
    <div className={cn("relative w-full", className)}>
      <Search
        className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500"
        aria-hidden="true"
      />
      <input
        type="search"
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        disabled={disabled}
        className="h-9 w-full rounded-md border border-white/10 bg-surface-950/60 pl-9 pr-16 text-sm text-slate-100 placeholder:text-slate-500 transition-colors focus:border-accent-500/60 focus:outline-none focus:ring-2 focus:ring-accent-500/20 disabled:cursor-not-allowed disabled:opacity-60"
      />
      {value && !disabled ? (
        <button
          type="button"
          aria-label="Clear search"
          onClick={() => onChange("")}
          className="absolute right-2 top-1/2 grid size-6 -translate-y-1/2 place-items-center rounded text-slate-500 hover:bg-white/[0.06] hover:text-slate-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-400"
        >
          <X className="size-3.5" aria-hidden="true" />
        </button>
      ) : shortcut ? (
        <kbd className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 font-mono text-[10px] text-slate-500">
          {shortcut}
        </kbd>
      ) : null}
    </div>
  );
}
