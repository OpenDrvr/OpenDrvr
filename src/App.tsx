import {
  CircleAlert,
  CircleCheck,
  Cpu,
  Download,
  HardDrive,
  RefreshCw,
  Search,
  ShieldCheck,
  Terminal,
} from "lucide-react";
import { useEffect, useState } from "react";
import { getAppInfo, type AppInfo } from "./lib/tauri";

type BackendState =
  | { status: "checking" }
  | { status: "connected"; info: AppInfo }
  | { status: "unavailable"; message: string };

const plannedCapabilities = [
  {
    icon: Cpu,
    title: "Hardware identification",
    description: "Enumerate the CPU, GPU, chipset, network and storage devices in your system.",
  },
  {
    icon: Search,
    title: "Driver discovery",
    description: "Match detected hardware against driver catalogs published by manufacturers.",
  },
  {
    icon: Download,
    title: "Official downloads",
    description: "Open verified download pages or packages directly from the official source.",
  },
] as const;

function describeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "The Rust backend did not respond to the IPC request.";
}

function BrandMark() {
  return (
    <div className="relative grid size-10 place-items-center rounded-xl bg-accent-500 shadow-[0_0_0_1px_rgba(147,197,253,0.35),0_10px_30px_-10px_rgba(59,130,246,0.9)]">
      <HardDrive className="size-5 text-white" strokeWidth={2.2} aria-hidden="true" />
      <span
        className="absolute -right-0.5 -bottom-0.5 size-3 rounded-full border-2 border-surface-950 bg-emerald-400"
        aria-hidden="true"
      />
    </div>
  );
}

function StatusPill({ state }: { state: BackendState }) {
  if (state.status === "checking") {
    return (
      <span
        className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.03] px-3 py-1.5 text-xs font-medium text-slate-300"
        role="status"
      >
        <RefreshCw className="size-3.5 animate-spin" aria-hidden="true" />
        Checking backend
      </span>
    );
  }

  if (state.status === "connected") {
    return (
      <span
        className="inline-flex items-center gap-2 rounded-full border border-emerald-400/20 bg-emerald-400/[0.07] px-3 py-1.5 text-xs font-medium text-emerald-300"
        role="status"
      >
        <span className="relative flex size-2" aria-hidden="true">
          <span className="absolute inline-flex size-full animate-ping rounded-full bg-emerald-400 opacity-60" />
          <span className="relative inline-flex size-2 rounded-full bg-emerald-400" />
        </span>
        Rust backend online
      </span>
    );
  }

  return (
    <span
      className="inline-flex items-center gap-2 rounded-full border border-amber-400/20 bg-amber-400/[0.07] px-3 py-1.5 text-xs font-medium text-amber-300"
      role="status"
    >
      <CircleAlert className="size-3.5" aria-hidden="true" />
      Backend unavailable
    </span>
  );
}

function InfoCell({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="min-w-0 bg-surface-900 px-4 py-3.5">
      <dt className="text-[0.65rem] font-semibold tracking-[0.16em] text-slate-500 uppercase">
        {label}
      </dt>
      <dd
        className={`mt-1 truncate text-sm text-slate-200 ${mono ? "font-mono text-[0.8rem]" : ""}`}
        title={value}
      >
        {value}
      </dd>
    </div>
  );
}

function AppInfoGrid({ info }: { info: AppInfo }) {
  return (
    <dl className="grid grid-cols-2 gap-px overflow-hidden rounded-xl border border-white/10 bg-white/10 sm:grid-cols-3 lg:grid-cols-5">
      <InfoCell label="Application" value={info.name} />
      <InfoCell label="Version" value={`v${info.version}`} mono />
      <InfoCell label="Platform" value={info.platform} mono />
      <InfoCell label="Architecture" value={info.architecture} mono />
      <InfoCell label="Build" value={info.profile} mono />
    </dl>
  );
}

function LoadingGrid() {
  return (
    <div
      className="grid animate-pulse grid-cols-2 gap-px overflow-hidden rounded-xl border border-white/10 bg-white/10 sm:grid-cols-3 lg:grid-cols-5"
      aria-label="Loading backend information"
    >
      {Array.from({ length: 5 }, (_, index) => (
        <div key={index} className="bg-surface-900 px-4 py-3.5">
          <div className="h-2 w-16 rounded bg-white/10" />
          <div className="mt-2.5 h-3.5 w-24 rounded bg-white/[0.07]" />
        </div>
      ))}
    </div>
  );
}

function ErrorPanel({ message }: { message: string }) {
  return (
    <div className="flex gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-4">
      <CircleAlert className="mt-0.5 size-4 shrink-0 text-amber-400" aria-hidden="true" />
      <div>
        <p className="text-sm font-medium text-amber-200">No IPC response</p>
        <p className="mt-1 text-sm leading-6 text-amber-100/70">{message}</p>
        <p className="mt-2 text-xs text-amber-100/50">
          Launch the desktop shell with{" "}
          <code className="font-mono text-amber-100/80">pnpm tauri:dev</code> to connect to Rust.
        </p>
      </div>
    </div>
  );
}

function BackendPanel() {
  const [state, setState] = useState<BackendState>({ status: "checking" });
  const isChecking = state.status === "checking";

  useEffect(() => {
    let active = true;

    getAppInfo().then(
      (info) => {
        if (active) {
          setState({ status: "connected", info });
        }
      },
      (error: unknown) => {
        if (active) {
          setState({ status: "unavailable", message: describeError(error) });
        }
      },
    );

    return () => {
      active = false;
    };
  }, []);

  const recheckBackend = () => {
    setState({ status: "checking" });

    getAppInfo().then(
      (info) => setState({ status: "connected", info }),
      (error: unknown) => setState({ status: "unavailable", message: describeError(error) }),
    );
  };

  return (
    <section
      className="mt-10 rounded-2xl border border-white/10 bg-surface-900/70 p-1 shadow-2xl shadow-black/30 backdrop-blur"
      aria-labelledby="backend-heading"
    >
      <div className="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <Terminal className="size-4 text-accent-400" aria-hidden="true" />
            <h2 id="backend-heading" className="text-sm font-semibold text-white">
              Rust backend connection
            </h2>
          </div>
          <p className="mt-1.5 text-sm leading-6 text-slate-400">
            React calls{" "}
            <code className="rounded bg-white/[0.06] px-1.5 py-0.5 font-mono text-[0.75rem] text-slate-300">
              invoke(&quot;get_app_info&quot;)
            </code>{" "}
            through the Tauri IPC bridge.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <StatusPill state={state} />
          <button
            type="button"
            onClick={recheckBackend}
            disabled={isChecking}
            className="inline-flex items-center gap-2 rounded-lg border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs font-medium text-slate-300 transition-colors hover:border-white/20 hover:bg-white/[0.07] hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-400 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <RefreshCw
              className={`size-3.5 ${isChecking ? "animate-spin" : ""}`}
              aria-hidden="true"
            />
            Recheck
          </button>
        </div>
      </div>

      <div className="px-5 pb-5">
        {state.status === "connected" ? (
          <AppInfoGrid info={state.info} />
        ) : state.status === "unavailable" ? (
          <ErrorPanel message={state.message} />
        ) : (
          <LoadingGrid />
        )}
      </div>
    </section>
  );
}

function App() {
  return (
    <div className="app-backdrop flex min-h-screen flex-col">
      <header className="border-b border-white/[0.06] bg-surface-950/70 backdrop-blur">
        <div className="mx-auto flex h-16 w-full max-w-5xl items-center justify-between px-6">
          <div className="flex items-center gap-3">
            <BrandMark />
            <div>
              <p className="text-sm font-semibold tracking-tight text-white">OpenDrvr</p>
              <p className="text-[0.7rem] text-slate-500">Open Driver · Windows utility</p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <span className="hidden items-center gap-1.5 rounded-full border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[0.68rem] font-medium text-slate-400 sm:inline-flex">
              Windows 10 / 11 · x64
            </span>
            <span className="inline-flex items-center gap-1.5 rounded-full border border-accent-400/20 bg-accent-400/[0.07] px-2.5 py-1 text-[0.68rem] font-medium text-accent-300">
              <ShieldCheck className="size-3" aria-hidden="true" />
              Phase 2
            </span>
          </div>
        </div>
      </header>

      <main className="mx-auto w-full max-w-5xl flex-1 px-6 py-12">
        <section className="max-w-3xl" aria-labelledby="hero-heading">
          <div className="inline-flex items-center gap-2 rounded-full border border-accent-400/20 bg-accent-400/[0.05] px-3 py-1 text-[0.7rem] font-medium text-accent-300">
            <ShieldCheck className="size-3.5" aria-hidden="true" />
            Open source · AGPL-3.0 licensed
          </div>

          <h1
            id="hero-heading"
            className="mt-5 text-4xl font-semibold tracking-[-0.035em] text-white sm:text-5xl"
          >
            Know your hardware.
            <br />
            <span className="text-slate-400">Trust your drivers.</span>
          </h1>

          <p className="mt-5 max-w-2xl text-base leading-7 text-slate-400">
            OpenDrvr identifies the components inside your PC and finds compatible drivers from
            official manufacturer sources. No bundled blobs, no guesswork.
          </p>
        </section>

        <BackendPanel />

        <section className="mt-10" aria-labelledby="roadmap-heading">
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <h2
              id="roadmap-heading"
              className="text-xs font-semibold tracking-[0.18em] text-slate-500 uppercase"
            >
              Foundation roadmap
            </h2>
            <span className="text-xs text-slate-600">Not implemented yet</span>
          </div>

          <div className="mt-4 grid gap-4 sm:grid-cols-3">
            {plannedCapabilities.map(({ icon: Icon, title, description }) => (
              <article
                key={title}
                className="rounded-xl border border-white/10 bg-surface-900/50 p-5 transition-colors hover:border-white/20"
              >
                <div className="flex items-center justify-between">
                  <span className="grid size-9 place-items-center rounded-lg border border-white/10 bg-white/[0.03]">
                    <Icon className="size-4 text-slate-300" aria-hidden="true" />
                  </span>
                  <span className="rounded-full border border-white/10 px-2 py-0.5 font-mono text-[0.65rem] text-slate-500">
                    Planned
                  </span>
                </div>
                <h3 className="mt-4 text-sm font-medium text-slate-100">{title}</h3>
                <p className="mt-1.5 text-sm leading-6 text-slate-500">{description}</p>
              </article>
            ))}
          </div>
        </section>

        <div className="mt-10 flex items-start gap-3 rounded-xl border border-white/10 bg-surface-900/40 p-4">
          <CircleCheck className="mt-0.5 size-4 shrink-0 text-emerald-400" aria-hidden="true" />
          <p className="text-sm leading-6 text-slate-400">
            <span className="font-medium text-slate-200">Phase 2 complete:</span> modular Rust
            backend with typed commands, validated configuration, structured logging and clear
            domain boundaries. Hardware detection, driver lookup and downloads are planned for later
            phases.
          </p>
        </div>
      </main>

      <footer className="border-t border-white/[0.06]">
        <div className="mx-auto flex w-full max-w-5xl flex-col gap-2 px-6 py-5 text-xs text-slate-600 sm:flex-row sm:items-center sm:justify-between">
          <p>OpenDrvr v0.1.0 · AGPL-3.0 · Tauri 2 + React + Rust</p>
          <p className="font-mono">x86_64-pc-windows-msvc</p>
        </div>
      </footer>
    </div>
  );
}

export default App;
