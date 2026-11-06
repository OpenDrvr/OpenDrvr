import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export type TooltipSide = "top" | "right" | "bottom" | "left";

const POSITION: Record<TooltipSide, string> = {
  top: "bottom-full left-1/2 mb-2 -translate-x-1/2",
  right: "left-full top-1/2 ml-2 -translate-y-1/2",
  bottom: "top-full left-1/2 mt-2 -translate-x-1/2",
  left: "right-full top-1/2 mr-2 -translate-y-1/2",
};

export interface TooltipProps {
  content: string;
  side?: TooltipSide;
  children: ReactNode;
  className?: string;
}

/** Lightweight hover/focus tooltip. The trigger keeps its own accessible name. */
export function Tooltip({ content, side = "top", children, className }: TooltipProps) {
  return (
    <span className={cn("group relative inline-flex", className)}>
      {children}
      <span
        aria-hidden="true"
        className={cn(
          "pointer-events-none absolute z-50 whitespace-nowrap rounded-md border border-white/10 bg-surface-800 px-2 py-1 text-[11px] font-medium text-slate-200 opacity-0 shadow-xl shadow-black/40 transition-opacity duration-100 group-focus-within:opacity-100 group-hover:opacity-100",
          POSITION[side],
        )}
      >
        {content}
      </span>
    </span>
  );
}
