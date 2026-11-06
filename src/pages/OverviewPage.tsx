import {
  CircleAlert,
  Database,
  Download,
  MonitorCog,
  RefreshCw,
  ScanSearch,
  ShieldCheck,
} from "lucide-react";
import { PageHeader, SectionHeader } from "../components/layout/PageHeader";
import {
  Button,
  LoadingState,
  Skeleton,
  StatusIndicator,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
  type StatusTone,
} from "../components/ui";
import type { PageId } from "../lib/navigation";
import type { BackendSnapshotState } from "../lib/useBackendSnapshot";
import type { ModuleState } from "../lib/tauri";

const CAPABILITIES = [
  {
    icon: MonitorCog,
    title: "Hardware identification",
    description:
      "Enumerate the processors, graphics adapters, chipsets, network and storage devices installed in this PC.",
  },
  {
    icon: ScanSearch,
    title: "Driver discovery",
    description:
      "Match detected hardware against the bundled catalog and driver metadata published by manufacturers.",
  },
  {
    icon: Download,
    title: "Download management",
    description:
      "Track official driver downloads, keep transfer history and open packages from verified sources.",
  },
] as const;

const PRINCIPLES = [
  {
    icon: MonitorCog,
    title: "Hardware first",
    description: "Start from the devices that are actually installed.",
  },
  {
    icon: ShieldCheck,
    title: "Official sources",
    description: "Link to manufacturer pages, not third-party bundles.",
  },
  {
    icon: Database,
    title: "Local by default",
    description: "Keep preferences and history on this computer.",
  },
] as const;

function moduleTone(state: ModuleState): StatusTone {
  if (state === "ready") {
    return "ready";
  }

  if (state === "planned") {
    return "planned";
  }

  return "unavailable";
}

function moduleLabel(state: ModuleState): string {
  if (state === "ready") {
    return "Ready";
  }

  if (state === "planned") {
    return "Planned";
  }

  return "Unavailable";
}

function titleCase(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function Detail({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <dt className="text-[10px] font-semibold uppercase tracking-[0.14em] text-slate-500">
        {label}
      </dt>
      <dd className={`mt-1 truncate text-xs text-slate-300 ${mono ? "font-mono" : ""}`}>{value}</dd>
    </div>
  );
}

export interface OverviewPageProps {
  backend: BackendSnapshotState;
  onRefresh: () => void;
  onNavigate: (page: PageId) => void;
}

export function OverviewPage({ backend, onRefresh, onNavigate }: OverviewPageProps) {
  const snapshot = backend.status === "ready" ? backend.snapshot : null;
  const isLoading = backend.status === "loading";

  return (
    <div className="space-y-8">
      <PageHeader
        title="Overview"
        description="A clear view of the OpenDrvr foundation and the capabilities planned for the next phases."
        actions={
          <Button
            variant="secondary"
            size="sm"
            leadingIcon={<RefreshCw className="size-3.5" aria-hidden="true" />}
            loading={isLoading}
            onClick={onRefresh}
          >
            Refresh status
          </Button>
        }
      />

      <section className="grid gap-4 lg:grid-cols-[1.45fr_1fr]">
        <div className="rounded-xl border border-white/[0.07] bg-surface-900/40 p-5">
          <div className="flex items-center gap-2">
            <ShieldCheck className="size-4 text-accent-300" aria-hidden="true" />
            <h2 className="text-sm font-semibold text-slate-100">
              Driver maintenance without guesswork
            </h2>
          </div>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-slate-400">
            OpenDrvr identifies the components inside your PC and connects them to driver downloads
            published by official manufacturer sources. No bundled driver blobs, no third-party
            repackaging, and no guesswork.
          </p>

          <div className="mt-4 grid gap-2 sm:grid-cols-3">
            {PRINCIPLES.map(({ icon: Icon, title, description }) => (
              <div
                key={title}
                className="rounded-lg border border-white/[0.06] bg-white/[0.02] p-3"
              >
                <Icon className="size-4 text-slate-400" aria-hidden="true" />
                <p className="mt-2 text-xs font-medium text-slate-200">{title}</p>
                <p className="mt-1 text-[11px] leading-5 text-slate-400">{description}</p>
              </div>
            ))}
          </div>
        </div>

        <div className="rounded-xl border border-white/[0.07] bg-surface-900/40 p-5">
          <SectionHeader title="Application" description="Live values from the Rust backend." />

          {isLoading ? (
            <div className="mt-4 grid grid-cols-2 gap-4">
              {Array.from({ length: 4 }, (_, index) => (
                <div key={index}>
                  <Skeleton className="h-2 w-14" />
                  <Skeleton className="mt-2 h-3.5 w-20" />
                </div>
              ))}
            </div>
          ) : backend.status === "error" ? (
            <div className="mt-4 flex gap-3 rounded-lg border border-amber-400/20 bg-amber-400/[0.05] p-3">
              <CircleAlert className="mt-0.5 size-4 shrink-0 text-amber-400" aria-hidden="true" />
              <div className="min-w-0">
                <p className="text-xs font-medium text-amber-200">Backend unavailable</p>
                <p className="mt-1 text-xs leading-5 text-amber-100/70">{backend.message}</p>
                <Button size="sm" className="mt-2" onClick={onRefresh}>
                  Retry
                </Button>
              </div>
            </div>
          ) : (
            <div className="mt-4 space-y-4">
              <dl className="grid grid-cols-2 gap-4">
                <Detail label="Version" value={`v${snapshot?.app.version ?? "—"}`} mono />
                <Detail label="Platform" value={snapshot?.app.platform ?? "—"} mono />
                <Detail label="Architecture" value={snapshot?.app.architecture ?? "—"} mono />
                <Detail label="Build" value={snapshot?.app.profile ?? "—"} mono />
              </dl>
              <div className="flex items-center justify-between gap-3 border-t border-white/[0.06] pt-3">
                <span className="text-xs text-slate-400">Backend</span>
                <StatusIndicator tone="ready" label="Online" />
              </div>
            </div>
          )}
        </div>
      </section>

      <section className="space-y-3">
        <SectionHeader
          title="Capabilities"
          description="The core workflows OpenDrvr is being built around."
        />
        <div className="grid gap-3 md:grid-cols-3">
          {CAPABILITIES.map(({ icon: Icon, title, description }) => (
            <article
              key={title}
              className="rounded-xl border border-white/[0.07] bg-surface-900/40 p-4"
            >
              <div className="flex items-start justify-between gap-3">
                <span className="grid size-8 place-items-center rounded-lg border border-white/[0.08] bg-white/[0.03]">
                  <Icon className="size-4 text-slate-300" aria-hidden="true" />
                </span>
                <StatusIndicator tone="planned" label="Planned" />
              </div>
              <h3 className="mt-3 text-sm font-medium text-slate-100">{title}</h3>
              <p className="mt-1.5 text-xs leading-5 text-slate-400">{description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="space-y-3">
        <SectionHeader
          title="System status"
          description="Module lifecycle reported by the Rust backend."
          action={
            <Button variant="ghost" size="sm" onClick={() => onNavigate("settings")}>
              View settings
            </Button>
          }
        />

        {isLoading ? (
          <LoadingState label="Reading backend status…" />
        ) : backend.status === "error" ? (
          <div className="flex gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-4">
            <CircleAlert className="mt-0.5 size-4 shrink-0 text-amber-400" aria-hidden="true" />
            <div>
              <p className="text-sm font-medium text-amber-200">Status unavailable</p>
              <p className="mt-1 text-sm leading-6 text-amber-100/70">{backend.message}</p>
            </div>
          </div>
        ) : (
          <Table>
            <TableHead>
              <TableRow>
                <TableHeaderCell>Module</TableHeaderCell>
                <TableHeaderCell className="w-32">State</TableHeaderCell>
                <TableHeaderCell>Detail</TableHeaderCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {backend.snapshot.backend.modules.map((module) => (
                <TableRow key={module.name}>
                  <TableCell className="font-medium text-slate-200">
                    {titleCase(module.name)}
                  </TableCell>
                  <TableCell>
                    <StatusIndicator
                      tone={moduleTone(module.state)}
                      label={moduleLabel(module.state)}
                    />
                  </TableCell>
                  <TableCell className="text-slate-400">{module.detail}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </section>
    </div>
  );
}
