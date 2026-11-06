import { CircleAlert, FolderOpen, Info, RefreshCw, RotateCcw, Settings } from "lucide-react";
import { useState, type ReactNode } from "react";
import { PageHeader, SectionHeader } from "../components/layout/PageHeader";
import {
  Button,
  Dropdown,
  IconButton,
  Input,
  Skeleton,
  StatusIndicator,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../components/ui";
import { useToast } from "../lib/toast";
import type { BackendSnapshotState } from "../lib/useBackendSnapshot";

const THEME_OPTIONS = [
  { value: "dark", label: "Dark" },
  { value: "system", label: "Match system (planned)", disabled: true },
  { value: "light", label: "Light (planned)", disabled: true },
] as const;

const CHANNEL_OPTIONS = [
  { value: "stable", label: "Stable" },
  { value: "beta", label: "Beta (planned)", disabled: true },
] as const;

function PreferenceCard({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <section className="rounded-xl border border-white/[0.07] bg-surface-900/40">
      <div className="border-b border-white/[0.06] px-5 py-4">
        <SectionHeader title={title} description={description} />
      </div>
      <div className="space-y-5 px-5 py-4">{children}</div>
    </section>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-white/[0.05] py-2.5 last:border-0 last:pb-0 first:pt-0">
      <dt className="text-xs text-slate-400">{label}</dt>
      <dd className="truncate font-mono text-xs text-slate-300">{value}</dd>
    </div>
  );
}

export interface SettingsPageProps {
  backend: BackendSnapshotState;
  onRefresh: () => void;
  onAbout: () => void;
}

export function SettingsPage({ backend, onRefresh, onAbout }: SettingsPageProps) {
  const { push } = useToast();
  const [theme, setTheme] = useState("dark");
  const [channel, setChannel] = useState("stable");
  const [directory, setDirectory] = useState("Downloads");
  const [checkUpdates, setCheckUpdates] = useState(true);
  const [notifyDownloads, setNotifyDownloads] = useState(true);
  const [keepHistory, setKeepHistory] = useState(true);

  const snapshot = backend.status === "ready" ? backend.snapshot : null;

  const resetPreview = () => {
    setTheme("dark");
    setChannel("stable");
    setDirectory("Downloads");
    setCheckUpdates(true);
    setNotifyDownloads(true);
    setKeepHistory(true);
    push({
      tone: "info",
      title: "Preview preferences reset",
      description: "Defaults were restored for this session. Nothing was written to disk.",
    });
  };

  return (
    <div className="space-y-8">
      <PageHeader
        title="Settings"
        description="Application preferences, local storage and build details."
        badge={<StatusIndicator tone="planned" label="Preview" />}
        actions={
          <Button
            variant="ghost"
            size="sm"
            leadingIcon={<RotateCcw className="size-3.5" aria-hidden="true" />}
            onClick={resetPreview}
          >
            Reset preview
          </Button>
        }
      />

      <div className="flex items-start gap-3 rounded-xl border border-accent-400/20 bg-accent-500/[0.06] p-4">
        <Info className="mt-0.5 size-4 shrink-0 text-accent-300" aria-hidden="true" />
        <div className="min-w-0">
          <h2 className="text-sm font-medium text-accent-200">Preferences are a preview</h2>
          <p className="mt-1 max-w-3xl text-xs leading-5 text-slate-400">
            These controls demonstrate the settings layout. Changes apply to this session only and
            are not saved yet, so they will reset when OpenDrvr restarts.
          </p>
        </div>
      </div>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_21rem]">
        <div className="space-y-5">
          <PreferenceCard title="Appearance" description="How OpenDrvr presents its interface.">
            <Dropdown
              label="Theme"
              value={theme}
              onChange={setTheme}
              options={THEME_OPTIONS}
              hint="OpenDrvr is dark-first. Additional themes are not implemented yet."
            />
            <Switch
              label="Compact spacing"
              description="Reduce vertical padding in lists and tables. Planned for a later phase."
              checked={false}
              onChange={() => undefined}
              disabled
            />
          </PreferenceCard>

          <PreferenceCard
            title="Updates and downloads"
            description="Defaults for driver package transfers."
          >
            <Dropdown
              label="Update channel"
              value={channel}
              onChange={setChannel}
              options={CHANNEL_OPTIONS}
              hint="Only the stable channel is available in this build."
            />
            <Input
              label="Download folder"
              value={directory}
              onChange={(event) => setDirectory(event.target.value)}
              hint="Preview value only — a native folder picker arrives with download management."
              trailing={
                <IconButton
                  icon={FolderOpen}
                  label="Choose download folder"
                  tooltip="Folder picker not available yet"
                  tooltipSide="left"
                  size="sm"
                  disabled
                />
              }
            />
            <Switch
              label="Check for catalog updates on launch"
              description="Refresh driver metadata when OpenDrvr starts. Requires the update service."
              checked={checkUpdates}
              onChange={setCheckUpdates}
            />
            <Switch
              label="Notify when a download completes"
              description="Show a desktop notification after a package finishes transferring."
              checked={notifyDownloads}
              onChange={setNotifyDownloads}
            />
            <Switch
              label="Keep download history"
              description="Retain a local record of completed transfers until you clear it."
              checked={keepHistory}
              onChange={setKeepHistory}
            />
          </PreferenceCard>
        </div>

        <div className="space-y-5">
          <section className="rounded-xl border border-white/[0.07] bg-surface-900/40">
            <div className="flex items-start justify-between gap-3 border-b border-white/[0.06] px-5 py-4">
              <SectionHeader title="Local data" description="Live values from the backend." />
              <IconButton
                icon={RefreshCw}
                label="Refresh local data"
                tooltip="Refresh local data"
                tooltipSide="left"
                size="sm"
                onClick={onRefresh}
                disabled={backend.status === "loading"}
              />
            </div>

            <div className="px-5 py-4">
              {backend.status === "loading" ? (
                <div className="space-y-3">
                  {Array.from({ length: 3 }, (_, index) => (
                    <div key={index} className="flex items-center justify-between gap-4">
                      <Skeleton className="h-3 w-24" />
                      <Skeleton className="h-3 w-12" />
                    </div>
                  ))}
                </div>
              ) : backend.status === "error" ? (
                <div className="flex gap-3 rounded-lg border border-amber-400/20 bg-amber-400/[0.05] p-3">
                  <CircleAlert
                    className="mt-0.5 size-4 shrink-0 text-amber-400"
                    aria-hidden="true"
                  />
                  <div className="min-w-0">
                    <p className="text-xs font-medium text-amber-200">Local data unavailable</p>
                    <p className="mt-1 text-xs leading-5 text-amber-100/70">{backend.message}</p>
                    <Button size="sm" className="mt-2" onClick={onRefresh}>
                      Retry
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="space-y-4">
                  <Table containerClassName="border-white/[0.05] bg-transparent">
                    <TableHead>
                      <TableRow>
                        <TableHeaderCell className="px-0">Store</TableHeaderCell>
                        <TableHeaderCell className="px-0">Schema</TableHeaderCell>
                        <TableHeaderCell className="px-0 text-right">State</TableHeaderCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      <TableRow className="hover:bg-transparent">
                        <TableCell className="px-0 text-xs text-slate-300">User database</TableCell>
                        <TableCell className="px-0 font-mono text-xs text-slate-400">
                          v{backend.snapshot.database.user.schemaVersion}
                        </TableCell>
                        <TableCell className="px-0 text-right">
                          <StatusIndicator
                            tone={
                              backend.snapshot.database.user.status === "ok"
                                ? "ready"
                                : "unavailable"
                            }
                            label={
                              backend.snapshot.database.user.status === "ok"
                                ? "Writable"
                                : "Unavailable"
                            }
                          />
                        </TableCell>
                      </TableRow>
                      <TableRow className="hover:bg-transparent">
                        <TableCell className="px-0 text-xs text-slate-300">
                          Bundled catalog
                        </TableCell>
                        <TableCell className="px-0 font-mono text-xs text-slate-400">
                          {backend.snapshot.database.catalog.schemaVersion === null
                            ? "—"
                            : `v${backend.snapshot.database.catalog.schemaVersion}`}
                        </TableCell>
                        <TableCell className="px-0 text-right">
                          <StatusIndicator
                            tone={
                              backend.snapshot.database.catalog.status === "ok"
                                ? "ready"
                                : "unavailable"
                            }
                            label={
                              backend.snapshot.database.catalog.status === "ok"
                                ? "Read-only"
                                : "Unavailable"
                            }
                          />
                        </TableCell>
                      </TableRow>
                    </TableBody>
                  </Table>

                  <dl>
                    <DetailRow
                      label="User tables"
                      value={String(backend.snapshot.database.user.tables)}
                    />
                    <DetailRow
                      label="Catalog tables"
                      value={String(backend.snapshot.database.catalog.tables)}
                    />
                    <DetailRow
                      label="Manufacturers"
                      value={String(backend.snapshot.database.catalog.manufacturerCount)}
                    />
                  </dl>
                </div>
              )}
            </div>
          </section>

          <section className="rounded-xl border border-white/[0.07] bg-surface-900/40">
            <div className="border-b border-white/[0.06] px-5 py-4">
              <SectionHeader title="Application" description="Build information." />
            </div>
            <div className="px-5 py-4">
              <dl>
                <DetailRow label="Version" value={snapshot ? `v${snapshot.app.version}` : "—"} />
                <DetailRow label="Platform" value={snapshot?.app.platform ?? "—"} />
                <DetailRow label="Architecture" value={snapshot?.app.architecture ?? "—"} />
                <DetailRow label="Build" value={snapshot?.app.profile ?? "—"} />
                <DetailRow label="License" value="AGPL-3.0" />
              </dl>
              <Button
                variant="secondary"
                size="sm"
                className="mt-4 w-full"
                leadingIcon={<Settings className="size-3.5" aria-hidden="true" />}
                onClick={onAbout}
              >
                About OpenDrvr
              </Button>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
