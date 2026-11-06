import { X } from "lucide-react";
import { useEffect, useId, useRef, type ReactNode } from "react";
import { IconButton } from "./IconButton";

export interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: ReactNode;
  footer?: ReactNode;
}

/** Native modal dialog: handles focus trapping, Escape and backdrop semantics. */
export function Dialog({ open, onClose, title, description, children, footer }: DialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const descriptionId = useId();

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }

    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }

    const handleCancel = (event: Event) => {
      event.preventDefault();
      onClose();
    };

    dialog.addEventListener("cancel", handleCancel);
    return () => dialog.removeEventListener("cancel", handleCancel);
  }, [onClose]);

  return (
    <dialog
      ref={dialogRef}
      aria-labelledby={titleId}
      aria-describedby={description ? descriptionId : undefined}
      onClose={onClose}
      className="m-auto w-[min(92vw,34rem)] rounded-xl border border-white/10 bg-surface-900 p-0 text-slate-200 shadow-2xl shadow-black/60 backdrop:bg-black/60"
    >
      <div className="flex items-start justify-between gap-4 border-b border-white/[0.06] px-5 py-4">
        <div className="min-w-0">
          <h2 id={titleId} className="text-sm font-semibold text-white">
            {title}
          </h2>
          {description && (
            <p id={descriptionId} className="mt-1 text-xs leading-5 text-slate-400">
              {description}
            </p>
          )}
        </div>
        <IconButton icon={X} label="Close dialog" size="sm" onClick={onClose} />
      </div>
      <div className="px-5 py-4">{children}</div>
      {footer && (
        <div className="flex items-center justify-end gap-2 border-t border-white/[0.06] px-5 py-3">
          {footer}
        </div>
      )}
    </dialog>
  );
}
