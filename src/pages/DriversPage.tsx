import { BadgeCheck, Database, Download, ScanSearch, ShieldCheck } from "lucide-react";
import { PageHeader, SectionHeader } from "../components/layout/PageHeader";
import { EmptyState, StatusIndicator } from "../components/ui";

const WORKFLOW = [
  {
    step: "01",
    icon: ScanSearch,
    title: "Identify",
    description: "Read the hardware identifiers reported by Windows for each installed device.",
  },
  {
    step: "02",
    icon: Database,
    title: "Match",
    description: "Compare those identifiers against the bundled catalog and manufacturer metadata.",
  },
  {
    step: "03",
    icon: Download,
    title: "Download",
    description: "Open the official manufacturer download page or package for the matched driver.",
  },
] as const;

export function DriversPage() {
  return (
    <div className="space-y-8">
      <PageHeader
        title="Drivers"
        description="Compatible drivers discovered from official manufacturer sources."
        badge={<StatusIndicator tone="planned" label="Planned" />}
      />

      <EmptyState
        icon={ScanSearch}
        title="Driver discovery is not available yet"
        description="Once hardware scanning exists, OpenDrvr will match detected devices against the bundled catalog. No driver statuses are reported on this page because none have been collected."
        badge={<StatusIndicator tone="planned" label="Planned for a later phase" />}
      />

      <section className="space-y-3">
        <SectionHeader
          title="How discovery will work"
          description="Every result will be traceable back to its source."
        />
        <div className="grid gap-3 md:grid-cols-3">
          {WORKFLOW.map(({ step, icon: Icon, title, description }) => (
            <article
              key={step}
              className="rounded-xl border border-white/[0.07] bg-surface-900/40 p-4"
            >
              <div className="flex items-center justify-between gap-3">
                <span className="grid size-8 place-items-center rounded-lg border border-white/[0.08] bg-white/[0.03]">
                  <Icon className="size-4 text-slate-300" aria-hidden="true" />
                </span>
                <span className="font-mono text-[11px] text-slate-500">{step}</span>
              </div>
              <h3 className="mt-3 text-sm font-medium text-slate-100">{title}</h3>
              <p className="mt-1.5 text-xs leading-5 text-slate-400">{description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="grid gap-3 md:grid-cols-2">
        <div className="flex items-start gap-3 rounded-xl border border-white/[0.07] bg-surface-900/40 p-4">
          <BadgeCheck className="mt-0.5 size-4 shrink-0 text-accent-300" aria-hidden="true" />
          <div className="min-w-0">
            <h2 className="text-sm font-medium text-slate-100">Official sources only</h2>
            <p className="mt-1 text-xs leading-5 text-slate-400">
              Driver links will point to manufacturer pages and packages. OpenDrvr does not
              redistribute drivers or bundle third-party installers.
            </p>
          </div>
        </div>
        <div className="flex items-start gap-3 rounded-xl border border-white/[0.07] bg-surface-900/40 p-4">
          <ShieldCheck className="mt-0.5 size-4 shrink-0 text-accent-300" aria-hidden="true" />
          <div className="min-w-0">
            <h2 className="text-sm font-medium text-slate-100">No automatic installation</h2>
            <p className="mt-1 text-xs leading-5 text-slate-400">
              You stay in control of every download and every installer. OpenDrvr will never change
              a device driver without an explicit action.
            </p>
          </div>
        </div>
      </section>
    </div>
  );
}
