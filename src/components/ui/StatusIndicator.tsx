import { cn } from "../../lib/cn";

export type StatusTone = "ready" | "planned" | "unavailable" | "neutral";

const TONES: Record<StatusTone, { badge: string; dot: string; text: string }> = {
  ready: {
    badge: "border-emerald-400/20 bg-emerald-400/[0.07] text-emerald-300",
    dot: "bg-emerald-400",
    text: "text-emerald-300",
  },
  planned: {
    badge: "border-accent-400/20 bg-accent-400/[0.07] text-accent-300",
    dot: "bg-accent-400",
    text: "text-accent-300",
  },
  unavailable: {
    badge: "border-amber-400/20 bg-amber-400/[0.07] text-amber-300",
    dot: "bg-amber-400",
    text: "text-amber-300",
  },
  neutral: {
    badge: "border-white/10 bg-white/[0.04] text-slate-400",
    dot: "bg-slate-500",
    text: "text-slate-400",
  },
};

export function StatusIndicator({
  tone,
  label,
  className,
}: {
  tone: StatusTone;
  label: string;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[11px] font-medium",
        TONES[tone].badge,
        className,
      )}
    >
      <span className={cn("size-1.5 rounded-full", TONES[tone].dot)} aria-hidden="true" />
      {label}
    </span>
  );
}

export function StatusDot({ tone, className }: { tone: StatusTone; className?: string }) {
  return (
    <span
      className={cn("size-1.5 shrink-0 rounded-full", TONES[tone].dot, className)}
      aria-hidden="true"
    />
  );
}

export function StatusText({ tone, children }: { tone: StatusTone; children: string }) {
  return <span className={cn("text-xs font-medium", TONES[tone].text)}>{children}</span>;
}
