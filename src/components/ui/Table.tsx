import type { ReactNode, TableHTMLAttributes, TdHTMLAttributes, ThHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

export interface TableProps extends TableHTMLAttributes<HTMLTableElement> {
  containerClassName?: string;
}

export function Table({ className, containerClassName, children, ...props }: TableProps) {
  return (
    <div
      className={cn(
        "overflow-hidden rounded-lg border border-white/[0.07] bg-surface-900/40",
        containerClassName,
      )}
    >
      <div className="overflow-x-auto">
        <table className={cn("w-full border-collapse text-left text-sm", className)} {...props}>
          {children}
        </table>
      </div>
    </div>
  );
}

export function TableHead({ children }: { children: ReactNode }) {
  return (
    <thead className="border-b border-white/[0.06] bg-white/[0.02] text-[10px] font-semibold uppercase tracking-[0.12em] text-slate-400">
      {children}
    </thead>
  );
}

export function TableBody({ children }: { children: ReactNode }) {
  return <tbody className="divide-y divide-white/[0.05]">{children}</tbody>;
}

export function TableRow({
  children,
  className,
  selected = false,
  onClick,
}: {
  children: ReactNode;
  className?: string;
  selected?: boolean;
  onClick?: () => void;
}) {
  return (
    <tr
      className={cn(
        "transition-colors hover:bg-white/[0.02]",
        onClick &&
          "cursor-pointer focus-visible:bg-white/[0.04] focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent-400",
        selected && "bg-accent-500/[0.08] hover:bg-accent-500/[0.1]",
        className,
      )}
      onClick={onClick}
      onKeyDown={
        onClick
          ? (event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onClick();
              }
            }
          : undefined
      }
      tabIndex={onClick ? 0 : undefined}
      aria-selected={onClick ? selected : undefined}
    >
      {children}
    </tr>
  );
}

export function TableHeaderCell({
  children,
  className,
  ...props
}: ThHTMLAttributes<HTMLTableCellElement>) {
  return (
    <th scope="col" className={cn("px-4 py-2.5 font-semibold", className)} {...props}>
      {children}
    </th>
  );
}

export function TableCell({
  children,
  className,
  ...props
}: TdHTMLAttributes<HTMLTableCellElement>) {
  return (
    <td className={cn("px-4 py-3 align-middle text-slate-300", className)} {...props}>
      {children}
    </td>
  );
}

export function TableEmpty({ colSpan, children }: { colSpan: number; children: ReactNode }) {
  return (
    <tr>
      <td colSpan={colSpan} className="px-4 py-4">
        {children}
      </td>
    </tr>
  );
}
