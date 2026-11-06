import { CircleCheck, Download, FolderOpen, History } from "lucide-react";
import { PageHeader, SectionHeader } from "../components/layout/PageHeader";
import { EmptyState, StatusIndicator } from "../components/ui";

const PLANNED_BEHAVIOUR = [
  {
    icon: Download,
    title: "Verified transfers",
    description:
      "Follow the official download URL and keep the package intact from source to disk.",
  },
  {
    icon: FolderOpen,
    title: "Clear destination",
    description: "Show exactly where each package is stored so nothing is hidden from you.",
  },
  {
    icon: History,
    title: "Transfer history",
    description: "Keep a local record of completed downloads that you can review or clear.",
  },
] as const;

export function DownloadsPage() {
  return (
    <div className="space-y-8">
      <PageHeader
        title="Downloads"
        description="Driver packages and transfer history for this computer."
        badge={<StatusIndicator tone="planned" label="Planned" />}
      />

      <EmptyState
        icon={Download}
        title="No downloads yet"
        description="Download management arrives in a later phase. When it does, packages requested from the Drivers page will appear here with their real status and destination."
        badge={<StatusIndicator tone="planned" label="Planned for a later phase" />}
      />

      <section className="space-y-3">
        <SectionHeader
          title="Planned behaviour"
          description="Downloads will stay visible and reversible."
        />
        <div className="grid gap-3 md:grid-cols-3">
          {PLANNED_BEHAVIOUR.map(({ icon: Icon, title, description }) => (
            <article
              key={title}
              className="rounded-xl border border-white/[0.07] bg-surface-900/40 p-4"
            >
              <span className="grid size-8 place-items-center rounded-lg border border-white/[0.08] bg-white/[0.03]">
                <Icon className="size-4 text-slate-300" aria-hidden="true" />
              </span>
              <h3 className="mt-3 text-sm font-medium text-slate-100">{title}</h3>
              <p className="mt-1.5 text-xs leading-5 text-slate-400">{description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="flex items-start gap-3 rounded-xl border border-white/[0.07] bg-surface-900/40 p-4">
        <CircleCheck className="mt-0.5 size-4 shrink-0 text-accent-300" aria-hidden="true" />
        <div className="min-w-0">
          <h2 className="text-sm font-medium text-slate-100">Nothing runs in the background</h2>
          <p className="mt-1 max-w-3xl text-xs leading-5 text-slate-400">
            There are no queued, paused or completed transfers because download management has not
            been implemented. This page will stay empty until real transfers exist.
          </p>
        </div>
      </section>
    </div>
  );
}
