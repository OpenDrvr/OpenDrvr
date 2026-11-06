import {
  Cpu,
  HardDrive,
  MemoryStick,
  MonitorCog,
  Network,
  ScanSearch,
  ShieldCheck,
} from "lucide-react";
import { PageHeader, SectionHeader } from "../components/layout/PageHeader";
import { EmptyState, StatusIndicator } from "../components/ui";

const PLANNED_CATEGORIES = [
  {
    icon: Cpu,
    title: "Processors",
    description: "Vendor, model, architecture and core topology.",
  },
  {
    icon: MonitorCog,
    title: "Graphics",
    description: "Discrete and integrated adapters with device identifiers.",
  },
  {
    icon: MemoryStick,
    title: "Memory",
    description: "Installed modules, capacity and configuration.",
  },
  {
    icon: HardDrive,
    title: "Storage",
    description: "Controllers, drives and interface type.",
  },
  {
    icon: Network,
    title: "Networking",
    description: "Wired, wireless and Bluetooth adapters.",
  },
  {
    icon: ScanSearch,
    title: "System devices",
    description: "Chipset, audio and other components reported by Windows.",
  },
] as const;

export function HardwarePage() {
  return (
    <div className="space-y-8">
      <PageHeader
        title="Hardware"
        description="A complete inventory of the components installed in this computer."
        badge={<StatusIndicator tone="planned" label="Planned" />}
      />

      <EmptyState
        icon={MonitorCog}
        title="Hardware scanning is not available yet"
        description="OpenDrvr will read device information from Windows and present it here. Scanning, grouping and export arrive in a later phase — nothing on this page is simulated."
        badge={<StatusIndicator tone="planned" label="Planned for a later phase" />}
      />

      <section className="space-y-3">
        <SectionHeader
          title="What will be detected"
          description="Inventory categories the hardware module is designed to cover."
        />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {PLANNED_CATEGORIES.map(({ icon: Icon, title, description }) => (
            <article
              key={title}
              className="rounded-xl border border-white/[0.07] bg-surface-900/40 p-4"
            >
              <div className="flex items-center gap-2.5">
                <span className="grid size-8 shrink-0 place-items-center rounded-lg border border-white/[0.08] bg-white/[0.03]">
                  <Icon className="size-4 text-slate-300" aria-hidden="true" />
                </span>
                <h3 className="text-sm font-medium text-slate-100">{title}</h3>
              </div>
              <p className="mt-2.5 text-xs leading-5 text-slate-400">{description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="flex items-start gap-3 rounded-xl border border-white/[0.07] bg-surface-900/40 p-4">
        <ShieldCheck className="mt-0.5 size-4 shrink-0 text-accent-300" aria-hidden="true" />
        <div className="min-w-0">
          <h2 className="text-sm font-medium text-slate-100">Read-only by design</h2>
          <p className="mt-1 max-w-3xl text-xs leading-5 text-slate-400">
            Hardware details will be collected locally for identification only. OpenDrvr does not
            modify devices, install drivers silently or send inventory data anywhere.
          </p>
        </div>
      </section>
    </div>
  );
}
