import { CircleAlert, CircleCheck, Copy, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { cn } from "../../lib/cn";
import type { BackendSnapshotState } from "../../lib/useBackendSnapshot";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { IconButton } from "../ui/IconButton";
import { Input } from "../ui/Input";
import { Spinner } from "../ui/Spinner";
import { StatusIndicator } from "../ui/StatusIndicator";

function Detail({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <dt className="text-[10px] font-semibold uppercase tracking-[0.14em] text-slate-500">
        {label}
      </dt>
      <dd className={cn("mt-1 truncate text-xs text-slate-300", mono && "font-mono")}>{value}</dd>
    </div>
  );
}

/** Copies text through the document command so WebView2 does not show a
 * browser clipboard permission prompt for a simple version string. */
function copyText(text: string): boolean {
  const previousFocus = document.activeElement;
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.top = "0";
  textarea.style.left = "0";
  textarea.style.opacity = "0";
  textarea.style.pointerEvents = "none";

  document.body.appendChild(textarea);
  textarea.focus();
  textarea.select();
  textarea.setSelectionRange(0, text.length);

  const copied = document.execCommand("copy");
  document.body.removeChild(textarea);

  if (previousFocus instanceof HTMLElement) {
    previousFocus.focus();
  }

  return copied;
}

export function AboutDialog({
  open,
  onClose,
  backend,
}: {
  open: boolean;
  onClose: () => void;
  backend: BackendSnapshotState;
}) {
  const snapshot = backend.status === "ready" ? backend.snapshot : null;
  const version = snapshot?.app.version ?? __APP_VERSION__;
  const [copyState, setCopyState] = useState<"idle" | "success" | "error">("idle");

  const handleClose = () => {
    setCopyState("idle");
    onClose();
  };

  const copyVersion = () => {
    setCopyState(copyText(`OpenDrvr v${version}`) ? "success" : "error");
  };

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      title="About OpenDrvr"
      description="Application details and local data status."
      footer={<Button onClick={handleClose}>Close</Button>}
    >
      <div className="space-y-5">
        <div className="flex items-center gap-3 rounded-lg border border-white/[0.07] bg-surface-950/50 p-3">
          <div className="grid size-10 place-items-center rounded-lg border border-accent-400/30 bg-accent-500/15">
            <ShieldCheck className="size-5 text-accent-300" aria-hidden="true" />
          </div>
          <div className="min-w-0">
            <p className="text-sm font-semibold text-white">OpenDrvr</p>
            <p className="mt-0.5 text-xs leading-5 text-slate-400">
              Open-source Windows utility for hardware identification and official driver discovery.
            </p>
          </div>
        </div>

        <Input
          label="Version"
          value={`OpenDrvr v${version}`}
          readOnly
          trailing={
            <IconButton
              icon={Copy}
              label="Copy version"
              tooltip="Copy version"
              tooltipSide="left"
              size="sm"
              onClick={copyVersion}
            />
          }
        />

        <div aria-live="polite" className="min-h-4">
          {copyState === "success" && (
            <p className="flex items-center gap-1.5 text-xs text-emerald-300">
              <CircleCheck className="size-3.5" aria-hidden="true" />
              Version copied to the clipboard.
            </p>
          )}
          {copyState === "error" && (
            <p className="flex items-center gap-1.5 text-xs text-amber-300">
              <CircleAlert className="size-3.5" aria-hidden="true" />
              Clipboard access was unavailable.
            </p>
          )}
        </div>

        <dl className="grid grid-cols-2 gap-x-5 gap-y-4">
          <Detail label="Platform" value={snapshot?.app.platform ?? "—"} mono />
          <Detail label="Architecture" value={snapshot?.app.architecture ?? "—"} mono />
          <Detail label="Build" value={snapshot?.app.profile ?? "—"} mono />
          <Detail label="License" value="AGPL-3.0" mono />
        </dl>

        <div className="space-y-3 border-t border-white/[0.06] pt-4">
          <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-slate-500">
            Local data
          </p>

          {backend.status === "loading" ? (
            <Spinner label="Loading local data status…" />
          ) : backend.status === "error" ? (
            <p className="text-xs leading-5 text-amber-300">{backend.message}</p>
          ) : (
            <div className="space-y-2.5">
              <div className="flex items-center justify-between gap-3">
                <span className="text-xs text-slate-400">User database</span>
                <div className="flex items-center gap-2">
                  <span className="font-mono text-[11px] text-slate-500">
                    schema v{backend.snapshot.database.user.schemaVersion}
                  </span>
                  <StatusIndicator
                    tone={backend.snapshot.database.user.status === "ok" ? "ready" : "unavailable"}
                    label={backend.snapshot.database.user.status === "ok" ? "Ready" : "Unavailable"}
                  />
                </div>
              </div>
              <div className="flex items-center justify-between gap-3">
                <span className="text-xs text-slate-400">Bundled catalog</span>
                <div className="flex items-center gap-2">
                  <span className="font-mono text-[11px] text-slate-500">
                    {backend.snapshot.database.catalog.schemaVersion === null
                      ? "schema —"
                      : `schema v${backend.snapshot.database.catalog.schemaVersion}`}
                  </span>
                  <StatusIndicator
                    tone={
                      backend.snapshot.database.catalog.status === "ok" ? "ready" : "unavailable"
                    }
                    label={
                      backend.snapshot.database.catalog.status === "ok"
                        ? "Read-only"
                        : "Unavailable"
                    }
                  />
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </Dialog>
  );
}
