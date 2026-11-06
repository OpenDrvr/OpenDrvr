import { HardDrive, Info, RefreshCw } from "lucide-react";
import { cn } from "../../lib/cn";
import { NAV_SECTIONS, type PageId } from "../../lib/navigation";
import type { BackendSnapshotState } from "../../lib/useBackendSnapshot";
import { IconButton } from "../ui/IconButton";
import { StatusDot, type StatusTone } from "../ui/StatusIndicator";

export interface SidebarProps {
  page: PageId;
  onNavigate: (page: PageId) => void;
  backend: BackendSnapshotState;
  onRefresh: () => void;
  onAbout: () => void;
}

function backendStatus(backend: BackendSnapshotState): { tone: StatusTone; label: string } {
  if (backend.status === "ready") {
    return { tone: "ready", label: "Backend online" };
  }

  if (backend.status === "loading") {
    return { tone: "neutral", label: "Checking backend" };
  }

  return { tone: "unavailable", label: "Backend offline" };
}

function dataStatus(backend: BackendSnapshotState): { tone: StatusTone; label: string } | null {
  if (backend.status !== "ready") {
    return null;
  }

  const { user, catalog } = backend.snapshot.database;
  if (user.status === "ok" && catalog.status === "ok") {
    return { tone: "ready", label: "Local data ready" };
  }

  if (user.status === "ok") {
    return { tone: "unavailable", label: "Catalog unavailable" };
  }

  return { tone: "unavailable", label: "Local data unavailable" };
}

export function Sidebar({ page, onNavigate, backend, onRefresh, onAbout }: SidebarProps) {
  const status = backendStatus(backend);
  const data = dataStatus(backend);
  const version = backend.status === "ready" ? backend.snapshot.app.version : __APP_VERSION__;

  return (
    <aside className="flex w-[236px] shrink-0 flex-col border-r border-white/[0.06] bg-surface-900/60">
      <div className="flex h-16 shrink-0 items-center gap-3 border-b border-white/[0.06] px-4">
        <div className="relative grid size-9 shrink-0 place-items-center rounded-lg border border-accent-400/30 bg-accent-500/15">
          <HardDrive className="size-[18px] text-accent-300" strokeWidth={2} aria-hidden="true" />
          <span
            className="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full border-2 border-surface-900 bg-accent-400"
            aria-hidden="true"
          />
        </div>
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold tracking-tight text-white">OpenDrvr</p>
          <p className="truncate text-[10px] font-medium uppercase tracking-[0.16em] text-slate-500">
            Windows utility
          </p>
        </div>
      </div>

      <nav className="min-h-0 flex-1 overflow-y-auto px-3 py-4" aria-label="Primary">
        <div className="space-y-5">
          {NAV_SECTIONS.map((section) => (
            <div key={section.label}>
              <p className="px-2 text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-500">
                {section.label}
              </p>
              <div className="mt-2 space-y-0.5">
                {section.pages.map((item) => {
                  const active = item.id === page;
                  const Icon = item.icon;

                  return (
                    <button
                      key={item.id}
                      type="button"
                      aria-current={active ? "page" : undefined}
                      onClick={() => onNavigate(item.id)}
                      className={cn(
                        "group relative flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-[13px] font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-400",
                        active
                          ? "bg-accent-500/[0.12] text-white"
                          : "text-slate-400 hover:bg-white/[0.04] hover:text-slate-200",
                      )}
                    >
                      {active && (
                        <span
                          className="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-accent-400"
                          aria-hidden="true"
                        />
                      )}
                      <Icon
                        className={cn(
                          "size-4 shrink-0",
                          active ? "text-accent-300" : "text-slate-500 group-hover:text-slate-400",
                        )}
                        strokeWidth={1.8}
                        aria-hidden="true"
                      />
                      <span className="truncate">{item.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </nav>

      <div className="shrink-0 border-t border-white/[0.06] p-3">
        <div className="rounded-lg border border-white/[0.07] bg-surface-950/50 p-3">
          <div className="flex items-center justify-between gap-2">
            <div className="flex min-w-0 items-center gap-2">
              <StatusDot tone={status.tone} />
              <span className="truncate text-xs font-medium text-slate-300">{status.label}</span>
            </div>
            <IconButton
              icon={RefreshCw}
              label="Refresh backend status"
              tooltip="Refresh backend status"
              tooltipSide="left"
              size="sm"
              onClick={onRefresh}
              disabled={backend.status === "loading"}
            />
          </div>

          {data && (
            <div className="mt-2 flex items-center gap-2 border-t border-white/[0.05] pt-2">
              <StatusDot tone={data.tone} />
              <span className="truncate text-[11px] text-slate-400">{data.label}</span>
            </div>
          )}

          <div className="mt-2 flex items-center justify-between gap-2 border-t border-white/[0.05] pt-2 text-[11px] text-slate-500">
            <span>OpenDrvr</span>
            <span className="font-mono">v{version}</span>
          </div>
        </div>

        <button
          type="button"
          onClick={onAbout}
          className="mt-2 flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-xs text-slate-400 transition-colors hover:bg-white/[0.04] hover:text-slate-300 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-400"
        >
          <Info className="size-3.5" aria-hidden="true" />
          About OpenDrvr
        </button>
      </div>
    </aside>
  );
}
