import { useMemo, useState, type ReactNode } from "react";
import {
  CircleAlert,
  CircleCheck,
  CircleOff,
  MonitorCog,
  RefreshCw,
  SearchX,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";
import { PageHeader, SectionHeader } from "../components/layout/PageHeader";
import {
  Button,
  Dropdown,
  EmptyState,
  LoadingState,
  SearchField,
  StatusIndicator,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
  Tabs,
  type StatusTone,
  type TabItem,
} from "../components/ui";
import { cn } from "../lib/cn";
import type {
  BusType,
  DeviceCategory,
  DeviceCondition,
  DeviceRecord,
  DeviceState,
  HardwareId,
  IdentifiedVendor,
  IdentifierFamily,
  IdentificationConfidence,
  ProblemKind,
} from "../lib/tauri";
import { useHardwareScan } from "../lib/useHardwareScan";

const CATEGORY_LABELS: Record<DeviceCategory, string> = {
  processor: "Processor",
  graphics: "Graphics",
  motherboard: "Motherboard",
  chipset: "Chipset",
  pci: "PCI device",
  usb: "USB device",
  network: "Network",
  "network-ethernet": "Ethernet",
  "network-wireless": "Wireless network",
  bluetooth: "Bluetooth",
  audio: "Audio",
  storage: "Storage",
  "storage-controller": "Storage controller",
  input: "Input device",
  unknown: "Unclassified",
};

const CONDITION_META: Record<DeviceCondition, { label: string; tone: StatusTone }> = {
  healthy: { label: "Healthy", tone: "ready" },
  disabled: { label: "Disabled", tone: "unavailable" },
  problem: { label: "Problem", tone: "unavailable" },
  "missing-driver": { label: "Missing driver", tone: "unavailable" },
  "non-present": { label: "Not connected", tone: "neutral" },
  unknown: { label: "Unknown", tone: "neutral" },
};

const CONFIDENCE_META: Record<
  IdentificationConfidence,
  { label: string; tone: StatusTone; hint: string }
> = {
  confirmed: {
    label: "Confirmed",
    tone: "ready",
    hint: "A known vendor identifier matched and Windows metadata does not conflict.",
  },
  partial: {
    label: "Partial",
    tone: "unavailable",
    hint: "Identity is incomplete, subsystem-only, or conflicts with Windows metadata.",
  },
  unresolved: {
    label: "Unresolved",
    tone: "neutral",
    hint: "No known vendor identifier matched. Reported values are shown as-is.",
  },
};

const IDENTIFIER_FAMILY_LABELS: Record<IdentifierFamily, string> = {
  pci: "PCI",
  usb: "USB",
  acpi: "ACPI",
  hdaudio: "HD Audio",
  other: "Other",
};

function vendorLabel(vendor: IdentifiedVendor | null): string | null {
  return vendor ? `${vendor.name} (${vendor.vendorId})` : null;
}

const STATE_LABELS: Record<DeviceState, string> = {
  started: "Started",
  "driver-loaded": "Driver loaded",
  stopped: "Stopped",
  disabled: "Disabled",
  problem: "Problem",
  unknown: "Unknown",
};

const PROBLEM_LABELS: Record<ProblemKind, string> = {
  "not-configured": "Not configured",
  "driver-load-failed": "Driver failed to load",
  "driver-blocked": "Driver blocked",
  "driver-unsigned": "Driver not signed",
  "device-disabled": "Device disabled",
  "hardware-disabled": "Hardware disabled",
  "device-not-present": "Device not present",
  "failed-start": "Device failed to start",
  "failed-install": "Driver installation failed",
  "needs-restart": "Restart required",
  "needs-reinstall": "Reinstall required",
  phantom: "Phantom device",
  "resource-conflict": "Resource conflict",
  other: "Other problem",
};

const ATTENTION_CONDITIONS: readonly DeviceCondition[] = ["disabled", "problem", "missing-driver"];

const NEEDS_RESTART_FLAG = 0x0000_0100;

function needsAttention(device: DeviceRecord): boolean {
  return device.isPresent && ATTENTION_CONDITIONS.includes(device.condition);
}

function deviceName(device: DeviceRecord): string {
  return device.friendlyName ?? device.deviceDescription ?? device.instanceId;
}

function formatBusType(busType: BusType): string {
  if (typeof busType === "object") {
    return busType.other;
  }

  const labels: Record<Exclude<BusType, { other: string }>, string> = {
    pci: "PCI",
    usb: "USB",
    acpi: "ACPI",
    root: "Root",
    "software-device": "Software device",
    bluetooth: "Bluetooth",
    audio: "Audio",
    storage: "Storage",
    unknown: "Unknown",
  };

  return labels[busType];
}

function matchesSearch(device: DeviceRecord, query: string): boolean {
  const identification = device.identification;
  const haystack = [
    device.friendlyName,
    device.deviceDescription,
    device.manufacturer,
    device.instanceId,
    device.className,
    device.driver?.provider,
    device.driver?.version,
    identification.manufacturer?.name,
    identification.manufacturer?.vendorId,
    identification.hardwareVendor?.name,
    identification.hardwareVendor?.vendorId,
    identification.subsystemVendor?.name,
    identification.subsystemVendor?.vendorId,
    identification.reportedManufacturer,
    identification.reportedName,
    ...identification.identifierFamilies,
    ...device.hardwareIds.map((identifier) => identifier.raw),
    ...device.compatibleIds.map((identifier) => identifier.raw),
  ]
    .filter((value): value is string => typeof value === "string" && value.length > 0)
    .join(" ")
    .toLowerCase();

  return haystack.includes(query);
}

function matchesConfidence(device: DeviceRecord, filter: string): boolean {
  return filter === "all" || device.identification.confidence === filter;
}

function describeIdentifier(identifier: HardwareId): string | null {
  const { kind } = identifier;

  switch (kind.bus) {
    case "pci": {
      const details = kind.details;
      const parts = [
        details.vendorId ? `VEN ${details.vendorId}` : null,
        details.deviceId ? `DEV ${details.deviceId}` : null,
        details.subsystemVendorId && details.subsystemDeviceId
          ? `SUBSYS ${details.subsystemVendorId}:${details.subsystemDeviceId}`
          : null,
        details.revision ? `REV ${details.revision}` : null,
        details.classCode ? `CLASS ${details.classCode}` : null,
      ];
      return parts.filter((part): part is string => part !== null).join(" · ") || null;
    }
    case "usb": {
      const details = kind.details;
      const parts = [
        details.vendorId ? `VID ${details.vendorId}` : null,
        details.productId ? `PID ${details.productId}` : null,
        details.revision ? `REV ${details.revision}` : null,
        details.interfaceNumber ? `MI ${details.interfaceNumber}` : null,
        details.classCode
          ? `CLASS ${details.classCode}${details.subclassCode ? `:${details.subclassCode}` : ""}${
              details.protocolCode ? `:${details.protocolCode}` : ""
            }`
          : null,
      ];
      return parts.filter((part): part is string => part !== null).join(" · ") || null;
    }
    case "acpi": {
      const details = kind.details;
      const parts = [
        details.vendorId ? `Vendor ${details.vendorId}` : null,
        details.deviceId ? `Device ${details.deviceId}` : null,
      ];
      return parts.filter((part): part is string => part !== null).join(" · ") || null;
    }
    case "hdaudio": {
      const details = kind.details;
      const parts = [
        details.functionGroup ? `FUNC ${details.functionGroup}` : null,
        details.vendorId ? `VEN ${details.vendorId}` : null,
        details.deviceId ? `DEV ${details.deviceId}` : null,
        details.subsystemVendorId && details.subsystemDeviceId
          ? `SUBSYS ${details.subsystemVendorId}:${details.subsystemDeviceId}`
          : null,
        details.revision ? `REV ${details.revision}` : null,
      ];
      return parts.filter((part): part is string => part !== null).join(" · ") || null;
    }
    case "unknown":
    default:
      return null;
  }
}

function matchesFilter(device: DeviceRecord, filter: string): boolean {
  switch (filter) {
    case "present":
      return device.isPresent;
    case "attention":
      return needsAttention(device);
    case "non-present":
      return !device.isPresent;
    case "all":
    default:
      return true;
  }
}

function SummaryCard({
  icon: Icon,
  label,
  value,
  hint,
  tone,
}: {
  icon: LucideIcon;
  label: string;
  value: number;
  hint: string;
  tone: string;
}) {
  return (
    <article className="rounded-xl border border-white/[0.07] bg-surface-900/40 p-4">
      <div className="flex items-center justify-between gap-3">
        <span className="grid size-8 place-items-center rounded-lg border border-white/[0.08] bg-white/[0.03]">
          <Icon className={cn("size-4", tone)} aria-hidden="true" />
        </span>
        <span className="font-mono text-xl font-semibold tracking-tight text-slate-100">
          {value}
        </span>
      </div>
      <h3 className="mt-3 text-sm font-medium text-slate-100">{label}</h3>
      <p className="mt-1 text-[11px] leading-5 text-slate-400">{hint}</p>
    </article>
  );
}

function DetailField({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string | null | undefined;
  mono?: boolean;
}) {
  return (
    <div className="min-w-0">
      <dt className="text-[10px] font-semibold uppercase tracking-[0.14em] text-slate-500">
        {label}
      </dt>
      <dd
        className={cn(
          "mt-1 break-words text-xs text-slate-300",
          mono && "font-mono text-[11px]",
          !value && "text-slate-500",
        )}
      >
        {value || "Not reported"}
      </dd>
    </div>
  );
}

function IdentifierList({ identifiers }: { identifiers: HardwareId[] }) {
  if (identifiers.length === 0) {
    return <p className="text-xs text-slate-500">Not reported</p>;
  }

  return (
    <ul className="space-y-2">
      {identifiers.map((identifier) => {
        const description = describeIdentifier(identifier);

        return (
          <li key={identifier.raw} className="min-w-0">
            <p className="break-all font-mono text-[11px] leading-5 text-slate-300">
              {identifier.raw}
            </p>
            {description ? (
              <p className="break-words font-mono text-[10px] leading-4 text-slate-500">
                {description}
              </p>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}

function DetailSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="space-y-3">
      <h3 className="text-[10px] font-semibold uppercase tracking-[0.14em] text-slate-500">
        {title}
      </h3>
      {children}
    </section>
  );
}

function DeviceDetail({ device }: { device: DeviceRecord }) {
  const condition = CONDITION_META[device.condition];
  const identification = device.identification;
  const confidence = CONFIDENCE_META[identification.confidence];
  const busAddress = [
    device.location.busNumber !== null ? `Bus ${device.location.busNumber}` : null,
    device.location.address !== null ? `Address ${device.location.address}` : null,
  ]
    .filter((value): value is string => value !== null)
    .join(" · ");
  const needsRestart = (device.status.flags & NEEDS_RESTART_FLAG) !== 0;
  const identifierFamilies = identification.identifierFamilies
    .map((family) => IDENTIFIER_FAMILY_LABELS[family])
    .join(", ");

  return (
    <section className="overflow-hidden rounded-xl border border-white/[0.07] bg-surface-900/40">
      <header className="flex flex-col gap-3 border-b border-white/[0.06] p-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <h2 className="break-words text-sm font-semibold text-slate-100">{deviceName(device)}</h2>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            {CATEGORY_LABELS[device.category]} · {device.isPresent ? "Present" : "Not connected"}
            {device.className ? ` · ${device.className}` : ""}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <StatusIndicator tone={confidence.tone} label={`Identity: ${confidence.label}`} />
          <StatusIndicator tone={condition.tone} label={condition.label} />
        </div>
      </header>

      <div className="grid gap-6 p-4 lg:grid-cols-2">
        <div className="space-y-5">
          <DetailSection title="Identification">
            <dl className="grid gap-3 sm:grid-cols-2">
              <DetailField
                label="Identified manufacturer"
                value={vendorLabel(identification.manufacturer)}
              />
              <DetailField
                label="Hardware vendor"
                value={vendorLabel(identification.hardwareVendor)}
              />
              <DetailField
                label="Subsystem vendor"
                value={vendorLabel(identification.subsystemVendor)}
              />
              <DetailField
                label="Windows manufacturer"
                value={identification.reportedManufacturer ?? device.manufacturer}
              />
              <DetailField
                label="Device name (reported)"
                value={identification.reportedName ?? deviceName(device)}
              />
              <DetailField label="Identification status" value={confidence.label} />
              <DetailField label="Identifier families" value={identifierFamilies || null} />
            </dl>
            <p className="text-[11px] leading-5 text-slate-500">{confidence.hint}</p>
            {identification.notes.length > 0 && (
              <ul className="space-y-1.5 rounded-lg border border-amber-400/15 bg-amber-400/[0.04] p-3">
                {identification.notes.map((note) => (
                  <li key={note} className="text-[11px] leading-5 text-amber-100/80">
                    {note}
                  </li>
                ))}
              </ul>
            )}
          </DetailSection>

          <DetailSection title="Identity">
            <dl className="grid gap-3 sm:grid-cols-2">
              <DetailField label="Device class" value={device.className} />
              <DetailField label="Enumerator" value={device.enumerator} />
              <DetailField label="Bus" value={formatBusType(device.busType)} />
              <DetailField label="Class GUID" value={device.classGuid} mono />
              <DetailField label="Parent" value={device.parentInstanceId} mono />
            </dl>
          </DetailSection>

          <DetailSection title="Instance ID">
            <p className="break-all font-mono text-[11px] leading-5 text-slate-300">
              {device.instanceId}
            </p>
          </DetailSection>

          <DetailSection title="Hardware IDs">
            <IdentifierList identifiers={device.hardwareIds} />
          </DetailSection>

          <DetailSection title="Compatible IDs">
            <IdentifierList identifiers={device.compatibleIds} />
          </DetailSection>
        </div>

        <div className="space-y-5">
          <DetailSection title="Driver">
            {device.driver ? (
              <dl className="grid gap-3 sm:grid-cols-2">
                <DetailField label="Provider" value={device.driver.provider} />
                <DetailField label="Version" value={device.driver.version} mono />
                <DetailField label="Date" value={device.driver.date} />
                <DetailField label="INF name" value={device.driver.infName} mono />
                <DetailField label="Service" value={device.driver.service} mono />
                <DetailField label="Description" value={device.driver.description} />
              </dl>
            ) : (
              <p className="rounded-lg border border-amber-400/15 bg-amber-400/[0.04] p-3 text-xs leading-5 text-amber-100/80">
                Windows did not report driver metadata for this device. It may be missing a driver,
                not require one, or expose its driver only through a parent device.
              </p>
            )}
          </DetailSection>

          <DetailSection title="Status">
            <dl className="grid gap-3 sm:grid-cols-2">
              <DetailField label="State" value={STATE_LABELS[device.status.state]} />
              <DetailField
                label="Problem"
                value={
                  device.status.problem
                    ? `${PROBLEM_LABELS[device.status.problem.kind]} (code ${device.status.problem.code})`
                    : "None reported"
                }
              />
              <DetailField
                label="Status flags"
                value={`0x${device.status.flags.toString(16).padStart(4, "0")}`}
                mono
              />
              <DetailField label="Restart required" value={needsRestart ? "Yes" : "No"} />
            </dl>
          </DetailSection>

          <DetailSection title="Location">
            <dl className="grid gap-3 sm:grid-cols-2">
              <DetailField label="Reported location" value={device.location.locationInfo} />
              <DetailField label="Bus / address" value={busAddress || null} />
            </dl>
            {device.location.locationPaths.length > 0 && (
              <ul className="mt-2 space-y-1">
                {device.location.locationPaths.map((path) => (
                  <li
                    key={path}
                    className="break-all font-mono text-[11px] leading-5 text-slate-300"
                  >
                    {path}
                  </li>
                ))}
              </ul>
            )}
          </DetailSection>
        </div>
      </div>
    </section>
  );
}

export function HardwarePage() {
  const { state, refresh } = useHardwareScan();
  const [search, setSearch] = useState("");
  const [conditionFilter, setConditionFilter] = useState("all");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [confidenceFilter, setConfidenceFilter] = useState("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const devices = useMemo(() => (state.status === "ready" ? state.snapshot.devices : []), [state]);

  const counts = useMemo(() => {
    const present = devices.filter((device) => device.isPresent).length;
    const attention = devices.filter(needsAttention).length;
    const healthy = devices.filter((device) => device.condition === "healthy").length;
    const confirmed = devices.filter(
      (device) => device.identification.confidence === "confirmed",
    ).length;
    const partial = devices.filter(
      (device) => device.identification.confidence === "partial",
    ).length;

    return {
      total: devices.length,
      present,
      healthy,
      attention,
      nonPresent: devices.length - present,
      confirmed,
      partial,
      unresolved: devices.length - confirmed - partial,
    };
  }, [devices]);

  const categories = useMemo(() => {
    const unique = new Set(devices.map((device) => device.category));
    return Array.from(unique).sort((left, right) =>
      CATEGORY_LABELS[left].localeCompare(CATEGORY_LABELS[right]),
    );
  }, [devices]);

  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();

    return devices.filter((device) => {
      if (categoryFilter !== "all" && device.category !== categoryFilter) {
        return false;
      }
      if (!matchesFilter(device, conditionFilter)) {
        return false;
      }
      if (!matchesConfidence(device, confidenceFilter)) {
        return false;
      }
      if (query.length > 0 && !matchesSearch(device, query)) {
        return false;
      }
      return true;
    });
  }, [devices, categoryFilter, conditionFilter, confidenceFilter, search]);

  const selectedDevice =
    filtered.find((device) => device.instanceId === selectedId) ?? filtered[0] ?? null;

  const tabs: TabItem[] = [
    { id: "all", label: "All devices", count: counts.total },
    { id: "present", label: "Present", count: counts.present },
    { id: "attention", label: "Needs attention", count: counts.attention },
    { id: "non-present", label: "Not connected", count: counts.nonPresent },
  ];

  const categoryOptions = [
    { value: "all", label: "All categories" },
    ...categories.map((category) => ({
      value: category,
      label: CATEGORY_LABELS[category],
    })),
  ];

  const confidenceOptions = [
    { value: "all", label: "All identities" },
    { value: "confirmed", label: "Confirmed" },
    { value: "partial", label: "Partial" },
    { value: "unresolved", label: "Unresolved" },
  ];

  const clearFilters = () => {
    setSearch("");
    setConditionFilter("all");
    setCategoryFilter("all");
    setConfidenceFilter("all");
  };

  const badge =
    state.status === "ready" ? (
      <StatusIndicator tone="ready" label="Live data" />
    ) : state.status === "error" ? (
      <StatusIndicator tone="unavailable" label="Unavailable" />
    ) : (
      <StatusIndicator tone="neutral" label="Scanning" />
    );

  return (
    <div className="space-y-6">
      <PageHeader
        title="Hardware"
        description="Real Plug and Play devices reported by Windows, with identifier-based identity, category, driver metadata and current status. Scanning is read-only and needs no administrator rights."
        badge={badge}
        actions={
          <Button
            variant="secondary"
            size="sm"
            leadingIcon={<RefreshCw className="size-3.5" aria-hidden="true" />}
            loading={state.status === "loading"}
            onClick={() => void refresh()}
          >
            Rescan hardware
          </Button>
        }
      />

      {state.status === "loading" ? (
        <LoadingState label="Scanning the Windows device tree…" />
      ) : state.status === "error" ? (
        <div className="flex gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-4">
          <CircleAlert className="mt-0.5 size-4 shrink-0 text-amber-400" aria-hidden="true" />
          <div className="min-w-0">
            <p className="text-sm font-medium text-amber-200">Hardware scan failed</p>
            <p className="mt-1 text-sm leading-6 text-amber-100/70">{state.message}</p>
            <Button size="sm" className="mt-3" onClick={() => void refresh()}>
              Retry scan
            </Button>
          </div>
        </div>
      ) : (
        <>
          <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <SummaryCard
              icon={MonitorCog}
              label="Present devices"
              value={counts.present}
              hint="Connected to this computer right now."
              tone="text-accent-300"
            />
            <SummaryCard
              icon={CircleCheck}
              label="Healthy"
              value={counts.healthy}
              hint="Started with no reported device problem."
              tone="text-emerald-300"
            />
            <SummaryCard
              icon={CircleAlert}
              label="Needs attention"
              value={counts.attention}
              hint="Disabled, missing a driver, or reporting an error."
              tone="text-amber-300"
            />
            <SummaryCard
              icon={CircleOff}
              label="Not connected"
              value={counts.nonPresent}
              hint="Installed previously but absent from the current tree."
              tone="text-slate-400"
            />
          </section>

          <section className="space-y-3">
            <SectionHeader
              title="Identification coverage"
              description="Identity is derived from hardware identifiers. Names reported by Windows are shown separately and are never treated as confirmed on their own."
            />
            <div className="grid gap-3 sm:grid-cols-3">
              <SummaryCard
                icon={ShieldCheck}
                label="Confirmed identity"
                value={counts.confirmed}
                hint="A known vendor ID matched and reported metadata does not conflict."
                tone="text-emerald-300"
              />
              <SummaryCard
                icon={CircleAlert}
                label="Partial identity"
                value={counts.partial}
                hint="Subsystem-only, incomplete, or conflicting identifier information."
                tone="text-amber-300"
              />
              <SummaryCard
                icon={CircleOff}
                label="Unresolved identity"
                value={counts.unresolved}
                hint="No known vendor ID matched; raw identifiers remain available."
                tone="text-slate-400"
              />
            </div>
          </section>

          {devices.length === 0 ? (
            <EmptyState
              icon={MonitorCog}
              title="No Plug and Play devices were reported"
              description="Windows returned an empty device tree. This is unusual; rescanning may help if a device was just connected."
              action={
                <Button size="sm" onClick={() => void refresh()}>
                  Rescan hardware
                </Button>
              }
            />
          ) : (
            <>
              <section className="space-y-3">
                <SectionHeader
                  title="Device inventory"
                  description="Select a device to inspect its identifiers, driver package and status."
                />

                <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                  <Tabs
                    tabs={tabs}
                    value={conditionFilter}
                    onChange={setConditionFilter}
                    ariaLabel="Filter devices by condition"
                    className="w-full overflow-x-auto lg:w-auto"
                  />
                  <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                    <SearchField
                      value={search}
                      onChange={setSearch}
                      placeholder="Search devices or IDs…"
                      className="sm:w-64"
                    />
                    <Dropdown
                      value={categoryFilter}
                      onChange={setCategoryFilter}
                      options={categoryOptions}
                      ariaLabel="Filter devices by category"
                      className="sm:w-44"
                    />
                    <Dropdown
                      value={confidenceFilter}
                      onChange={setConfidenceFilter}
                      options={confidenceOptions}
                      ariaLabel="Filter devices by identification confidence"
                      className="sm:w-40"
                    />
                  </div>
                </div>

                {filtered.length === 0 ? (
                  <EmptyState
                    icon={SearchX}
                    title="No devices match these filters"
                    description="Try a different search term, category, identity, or condition filter."
                    action={
                      <Button size="sm" onClick={clearFilters}>
                        Clear filters
                      </Button>
                    }
                  />
                ) : (
                  <>
                    <Table>
                      <TableHead>
                        <TableRow>
                          <TableHeaderCell>Device</TableHeaderCell>
                          <TableHeaderCell className="w-44">Identity</TableHeaderCell>
                          <TableHeaderCell className="w-44">Category</TableHeaderCell>
                          <TableHeaderCell className="w-36">Condition</TableHeaderCell>
                          <TableHeaderCell className="w-56">Driver</TableHeaderCell>
                        </TableRow>
                      </TableHead>
                      <TableBody>
                        {filtered.map((device) => {
                          const condition = CONDITION_META[device.condition];
                          const confidence = CONFIDENCE_META[device.identification.confidence];

                          return (
                            <TableRow
                              key={device.instanceId}
                              selected={device.instanceId === selectedDevice?.instanceId}
                              onClick={() => setSelectedId(device.instanceId)}
                            >
                              <TableCell>
                                <div className="min-w-0">
                                  <p className="truncate text-xs font-medium text-slate-100">
                                    {deviceName(device)}
                                  </p>
                                  <p className="mt-0.5 truncate font-mono text-[10px] text-slate-500">
                                    {device.instanceId}
                                  </p>
                                </div>
                              </TableCell>
                              <TableCell>
                                <div className="min-w-0">
                                  <p className="truncate text-xs text-slate-300">
                                    {device.identification.manufacturer?.name ?? "Unresolved"}
                                  </p>
                                  <p className="mt-0.5 truncate text-[10px] text-slate-500">
                                    {confidence.label}
                                  </p>
                                </div>
                              </TableCell>
                              <TableCell className="text-xs text-slate-400">
                                {CATEGORY_LABELS[device.category]}
                              </TableCell>
                              <TableCell>
                                <StatusIndicator tone={condition.tone} label={condition.label} />
                              </TableCell>
                              <TableCell className="text-xs text-slate-400">
                                {device.driver?.provider ?? "Not reported"}
                                {device.driver?.version ? (
                                  <span className="mt-0.5 block font-mono text-[10px] text-slate-500">
                                    {device.driver.version}
                                  </span>
                                ) : null}
                              </TableCell>
                            </TableRow>
                          );
                        })}
                      </TableBody>
                    </Table>

                    {selectedDevice && <DeviceDetail device={selectedDevice} />}
                  </>
                )}
              </section>

              <section className="flex flex-col gap-2 rounded-xl border border-white/[0.07] bg-surface-900/30 p-4 sm:flex-row sm:items-start sm:justify-between">
                <div className="flex items-start gap-3">
                  <ShieldCheck
                    className="mt-0.5 size-4 shrink-0 text-accent-300"
                    aria-hidden="true"
                  />
                  <p className="max-w-3xl text-xs leading-5 text-slate-400">
                    Values are read directly from Windows Configuration Manager. Identity is derived
                    from known vendor identifiers; unknown or incomplete IDs stay unresolved instead
                    of being guessed. OpenDrvr does not modify devices or install drivers from this
                    page.
                  </p>
                </div>
                <p className="shrink-0 font-mono text-[10px] text-slate-500">
                  {state.snapshot.diagnostics.provider} · {state.snapshot.diagnostics.durationMs} ms
                </p>
              </section>

              {(state.snapshot.diagnostics.propertyFailures.length > 0 ||
                state.snapshot.diagnostics.deviceFailures.length > 0) && (
                <p className="text-[11px] leading-5 text-slate-500">
                  {state.snapshot.diagnostics.propertyFailures.length} property read(s) and{" "}
                  {state.snapshot.diagnostics.deviceFailures.length} device(s) could not be read.
                  The remaining devices were still returned.
                </p>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}
