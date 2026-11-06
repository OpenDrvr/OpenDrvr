import type { ReactNode } from "react";

export interface PageHeaderProps {
  title: string;
  description: string;
  badge?: ReactNode;
  actions?: ReactNode;
}

export function PageHeader({ title, description, badge, actions }: PageHeaderProps) {
  return (
    <header className="flex flex-col gap-4 border-b border-white/[0.06] pb-6 md:flex-row md:items-start md:justify-between">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2.5">
          <h1 className="text-[1.35rem] font-semibold tracking-[-0.015em] text-white">{title}</h1>
          {badge}
        </div>
        <p className="mt-1.5 max-w-2xl text-sm leading-6 text-slate-400">{description}</p>
      </div>
      {actions && <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div>}
    </header>
  );
}

export function SectionHeader({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-slate-100">{title}</h2>
        {description && <p className="mt-0.5 text-xs leading-5 text-slate-400">{description}</p>}
      </div>
      {action}
    </div>
  );
}
