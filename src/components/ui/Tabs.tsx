import { useRef, type KeyboardEvent } from "react";
import { cn } from "../../lib/cn";

export interface TabItem {
  id: string;
  label: string;
  count?: number;
  disabled?: boolean;
}

export interface TabsProps {
  tabs: readonly TabItem[];
  value: string;
  onChange: (id: string) => void;
  ariaLabel: string;
  className?: string;
}

export function Tabs({ tabs, value, onChange, ariaLabel, className }: TabsProps) {
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});

  const focusTab = (tab: TabItem) => {
    onChange(tab.id);
    refs.current[tab.id]?.focus();
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>, current: TabItem) => {
    const enabled = tabs.filter((tab) => !tab.disabled);
    if (enabled.length === 0) {
      return;
    }

    const currentIndex = enabled.findIndex((tab) => tab.id === current.id);
    let next: TabItem | undefined;

    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      next = enabled[(currentIndex + 1) % enabled.length];
    } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      next = enabled[(currentIndex - 1 + enabled.length) % enabled.length];
    } else if (event.key === "Home") {
      next = enabled[0];
    } else if (event.key === "End") {
      next = enabled[enabled.length - 1];
    }

    if (next) {
      event.preventDefault();
      focusTab(next);
    }
  };

  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      className={cn(
        "inline-flex items-center gap-1 rounded-lg border border-white/[0.06] bg-surface-950/50 p-1",
        className,
      )}
    >
      {tabs.map((tab) => {
        const selected = tab.id === value;

        return (
          <button
            key={tab.id}
            ref={(node) => {
              refs.current[tab.id] = node;
            }}
            type="button"
            role="tab"
            aria-selected={selected}
            disabled={tab.disabled}
            tabIndex={selected ? 0 : -1}
            onClick={() => onChange(tab.id)}
            onKeyDown={(event) => handleKeyDown(event, tab)}
            className={cn(
              "inline-flex h-7 items-center gap-1.5 rounded-md px-2.5 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-400 disabled:cursor-not-allowed disabled:opacity-40",
              selected
                ? "bg-white/[0.08] text-white shadow-sm shadow-black/20"
                : "text-slate-400 hover:bg-white/[0.03] hover:text-slate-200",
            )}
          >
            {tab.label}
            {typeof tab.count === "number" && (
              <span className="rounded bg-white/[0.06] px-1 font-mono text-[10px] text-slate-400">
                {tab.count}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
