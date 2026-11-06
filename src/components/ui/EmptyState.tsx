import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description: string;
  badge?: ReactNode;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  badge,
  action,
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center rounded-xl border border-dashed border-white/[0.08] bg-surface-900/30 px-6 py-12 text-center",
        className,
      )}
    >
      <div className="grid size-11 place-items-center rounded-lg border border-white/[0.08] bg-white/[0.03]">
        <Icon className="size-5 text-accent-300" aria-hidden="true" />
      </div>
      <h3 className="mt-4 text-sm font-semibold text-slate-100">{title}</h3>
      <p className="mt-1.5 max-w-md text-sm leading-6 text-slate-400">{description}</p>
      {badge && <div className="mt-4">{badge}</div>}
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}
