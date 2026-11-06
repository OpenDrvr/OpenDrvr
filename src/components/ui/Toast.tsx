import { CircleAlert, CircleCheck, Info, X } from "lucide-react";
import { useCallback, useRef, useState, type ReactNode } from "react";
import { cn } from "../../lib/cn";
import { ToastContext, type ToastInput, type ToastRecord, type ToastTone } from "../../lib/toast";
import { IconButton } from "./IconButton";

const ICONS = {
  info: Info,
  success: CircleCheck,
  error: CircleAlert,
} as const;

const TONES: Record<ToastTone, string> = {
  info: "border-white/10 bg-surface-850 text-slate-300",
  success: "border-emerald-400/20 bg-surface-850 text-emerald-300",
  error: "border-amber-400/20 bg-surface-850 text-amber-300",
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastRecord[]>([]);
  const nextId = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const push = useCallback(
    (input: ToastInput) => {
      const id = ++nextId.current;
      const record: ToastRecord = {
        id,
        tone: input.tone ?? "info",
        title: input.title,
        ...(input.description ? { description: input.description } : {}),
      };

      setToasts((current) => [...current, record]);
      window.setTimeout(() => dismiss(id), 5000);
    },
    [dismiss],
  );

  return (
    <ToastContext.Provider value={{ push }}>
      {children}
      <div
        aria-live="polite"
        className="pointer-events-none fixed bottom-4 right-4 z-[100] flex w-[min(92vw,22rem)] flex-col gap-2"
      >
        {toasts.map((toast) => {
          const Icon = ICONS[toast.tone];

          return (
            <div
              key={toast.id}
              role={toast.tone === "error" ? "alert" : "status"}
              className={cn(
                "pointer-events-auto flex items-start gap-3 rounded-lg border p-3 shadow-xl shadow-black/40 backdrop-blur",
                TONES[toast.tone],
              )}
            >
              <Icon className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              <div className="min-w-0 flex-1">
                <p className="text-xs font-semibold text-slate-100">{toast.title}</p>
                {toast.description && (
                  <p className="mt-1 text-xs leading-5 text-slate-400">{toast.description}</p>
                )}
              </div>
              <IconButton
                icon={X}
                label="Dismiss notification"
                size="sm"
                onClick={() => dismiss(toast.id)}
              />
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}
