import {
  CircleAlert,
  CircleCheck,
  FolderOpen,
  Info,
  RefreshCw,
  RotateCcw,
  Settings,
} from "lucide-react";
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
import { describeError } from "../lib/errors";
import type { AppPreferences, ThemePreference, UpdateChannel } from "../lib/tauri";
import { useToast } from "../lib/toast";
import { usePreferences } from "../lib/usePreferences";
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

function preferencesEqual(a: AppPreferences, b: AppPreferences): boolean {
  return (
    a.theme === b.theme &&
    a.updateChannel === b.updateChannel &&
    a.downloadDirectory === b.downloadDirectory &&
    a.checkCatalogUpdates === b.checkCatalogUpdates &&
    a.notifyDownloads === b.notifyDownloads &&
    a.keepDownloadHistory === b.keepDownloadHistory
  );
}

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

function PreferencesLoading() {
  return (
    <div className="space-y-5" aria-busy="true">
      {[0, 1].map((card) => (
        <section key={card} className="rounded-xl border border-white/[0.07] bg-surface-900/40">
          <div className="border-b border-white/[0.06] px-5 py-4">
            <Skeleton className="h-4 w-28" />
            <Skeleton className="mt-2 h-3 w-56" />
          </div>
          <div className="space-y-5 px-5 py-4">
            {Array.from({ length: card === 0 ? 2 : 4 }, (_, index) => (
              <div key={index}>
                <Skeleton className="h-3 w-20" />
                <Skeleton className="mt-2 h-9 w-full" />
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

function PreferencesError({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div
      role="alert"
      className="flex gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-4"
    >
      <CircleAlert className="mt-0.5 size-4 shrink-0 text-amber-400" aria-hidden="true" />
      <div className="min-w-0">
        <p className="text-sm font-medium text-amber-200">Preferences unavailable</p>
        <p className="mt-1 text-xs leading-5 text-amber-100/70">{message}</p>
        <Button size="sm" className="mt-2" onClick={onRetry}>
          Retry
        </Button>
      </div>
    </div>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-white/[0.05] py-2.5 first:pt-0 last:border-0 last:pb-0">
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
  const {
    state: preferencesState,
    pendingAction,
    save,
    reset,
    refresh: refreshPreferences,
  } = usePreferences();
  const [draft, setDraft] = useState<AppPreferences | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  const snapshot = backend.status === "ready" ? backend.snapshot : null;
  const savedPreferences =
    preferencesState.status === "ready" ? preferencesState.preferences : null;
  const current = draft ?? savedPreferences;
  const isDirty =
    current !== null && savedPreferences !== null && !preferencesEqual(current, savedPreferences);
  const directoryError =
    current !== null && current.downloadDirectory.trim().length === 0
      ? "Enter a folder name or path."
      : undefined;
  const busy = pendingAction !== null;

  const updatePreference = <K extends keyof AppPreferences>(key: K, value: AppPreferences[K]) => {
    if (!current) {
      return;
    }

    setDraft({ ...current, [key]: value });
    setSaveError(null);
  };

  const discardChanges = () => {
    setDraft(null);
    setSaveError(null);
  };

  const handleSave = async () => {
    if (!current || !isDirty || directoryError) {
      return;
    }

    setSaveError(null);

    try {
      const saved = await save({
        ...current,
        downloadDirectory: current.downloadDirectory.trim(),
      });

      if (!saved) {
        return;
      }

      setDraft(null);
      push({
        tone: "success",
        title: "Preferences saved",
        description: "Your settings are stored in the local OpenDrvr database.",
      });
    } catch (error) {
      const message = describeError(error);
      setSaveError(message);
      push({
        tone: "error",
        title: "Could not save preferences",
        description: message,
      });
    }
  };

  const handleReset = async () => {
    setSaveError(null);

    try {
      const defaults = await reset();

      if (!defaults) {
        return;
      }

      setDraft(null);
      push({
        tone: "success",
        title: "Defaults restored",
        description: "OpenDrvr preferences were reset to their defaults.",
      });
    } catch (error) {
      const message = describeError(error);
      setSaveError(message);
      push({
        tone: "error",
        title: "Could not restore defaults",
        description: message,
      });
    }
  };

  return (
    <div className="space-y-8">
      <PageHeader
        title="Settings"
        description="Preferences, local storage and build details."
        actions={
          <Button
            variant="ghost"
            size="sm"
            leadingIcon={<RotateCcw className="size-3.5" aria-hidden="true" />}
            loading={pendingAction === "reset"}
            disabled={preferencesState.status !== "ready" || busy}
            onClick={handleReset}
          >
            Restore defaults
          </Button>
        }
      />

      <div className="flex items-start gap-3 rounded-xl border border-accent-400/20 bg-accent-500/[0.06] p-4">
        <Info className="mt-0.5 size-4 shrink-0 text-accent-300" aria-hidden="true" />
        <div className="min-w-0">
          <h2 className="text-sm font-medium text-accent-200">Preferences are stored locally</h2>
          <p className="mt-1 max-w-3xl text-xs leading-5 text-slate-400">
            Changes are written to the OpenDrvr user database on this computer and restored the next
            time the application starts.
          </p>
        </div>
      </div>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_21rem]">
        <div className="space-y-5">
          {preferencesState.status === "loading" ? (
            <PreferencesLoading />
          ) : preferencesState.status === "error" ? (
            <PreferencesError
              message={preferencesState.message}
              onRetry={() => void refreshPreferences()}
            />
          ) : current ? (
            <>
              <PreferenceCard title="Appearance" description="How OpenDrvr presents its interface.">
                <Dropdown
                  label="Theme"
                  value={current.theme}
                  onChange={(value) => updatePreference("theme", value as ThemePreference)}
                  options={THEME_OPTIONS}
                  hint="OpenDrvr is dark-first. Additional themes are planned."
                  disabled={busy}
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
                  value={current.updateChannel}
                  onChange={(value) => updatePreference("updateChannel", value as UpdateChannel)}
                  options={CHANNEL_OPTIONS}
                  hint="Only the stable channel is available in this build."
                  disabled={busy}
                />
                <Input
                  label="Download folder"
                  value={current.downloadDirectory}
                  onChange={(event) => updatePreference("downloadDirectory", event.target.value)}
                  {...(directoryError ? { error: directoryError } : {})}
                  hint="Stored locally. A native folder picker arrives with download management."
                  maxLength={260}
                  disabled={busy}
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
                  checked={current.checkCatalogUpdates}
                  onChange={(checked) => updatePreference("checkCatalogUpdates", checked)}
                  disabled={busy}
                />
                <Switch
                  label="Notify when a download completes"
                  description="Show a desktop notification after a package finishes transferring."
                  checked={current.notifyDownloads}
                  onChange={(checked) => updatePreference("notifyDownloads", checked)}
                  disabled={busy}
                />
                <Switch
                  label="Keep download history"
                  description="Retain a local record of completed transfers until you clear it."
                  checked={current.keepDownloadHistory}
                  onChange={(checked) => updatePreference("keepDownloadHistory", checked)}
                  disabled={busy}
                />
              </PreferenceCard>

              {saveError && (
                <div
                  role="alert"
                  className="flex gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-4"
                >
                  <CircleAlert
                    className="mt-0.5 size-4 shrink-0 text-amber-400"
                    aria-hidden="true"
                  />
                  <div className="min-w-0">
                    <p className="text-sm font-medium text-amber-200">Preferences were not saved</p>
                    <p className="mt-1 text-xs leading-5 text-amber-100/70">{saveError}</p>
                  </div>
                </div>
              )}

              <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-white/[0.07] bg-surface-900/40 px-4 py-3">
                <div className="flex min-w-0 items-start gap-2.5">
                  {isDirty ? (
                    <span
                      className="mt-1.5 size-2 shrink-0 rounded-full bg-amber-400"
                      aria-hidden="true"
                    />
                  ) : (
                    <CircleCheck
                      className="mt-0.5 size-4 shrink-0 text-emerald-400"
                      aria-hidden="true"
                    />
                  )}
                  <div className="min-w-0">
                    <p className="text-xs font-medium text-slate-200">
                      {isDirty ? "Unsaved changes" : "All changes saved"}
                    </p>
                    <p className="mt-0.5 text-[11px] text-slate-500">
                      {isDirty
                        ? "Save to write them to the local database."
                        : "Preferences match the local database."}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={discardChanges}
                    disabled={!isDirty || busy}
                  >
                    Discard
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={handleSave}
                    loading={pendingAction === "save"}
                    disabled={!isDirty || Boolean(directoryError) || busy}
                  >
                    Save changes
                  </Button>
                </div>
              </div>
            </>
          ) : null}
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
