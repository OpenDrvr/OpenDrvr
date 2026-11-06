import type { LucideIcon } from "lucide-react";
import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";
import { Tooltip, type TooltipSide } from "./Tooltip";

export type IconButtonVariant = "ghost" | "secondary";
export type IconButtonSize = "sm" | "md";

const VARIANTS: Record<IconButtonVariant, string> = {
  ghost: "text-slate-500 hover:bg-white/[0.06] hover:text-slate-200 active:bg-white/[0.04]",
  secondary:
    "border border-white/10 bg-white/[0.04] text-slate-300 hover:border-white/20 hover:bg-white/[0.07] hover:text-white active:bg-white/[0.05]",
};

const SIZES: Record<IconButtonSize, string> = {
  sm: "size-7",
  md: "size-8",
};

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: LucideIcon;
  label: string;
  tooltip?: string;
  tooltipSide?: TooltipSide;
  variant?: IconButtonVariant;
  size?: IconButtonSize;
}

export function IconButton({
  icon: Icon,
  label,
  tooltip,
  tooltipSide,
  variant = "ghost",
  size = "md",
  className,
  type = "button",
  ...props
}: IconButtonProps) {
  const button = (
    <button
      type={type}
      aria-label={label}
      className={cn(
        "inline-grid shrink-0 place-items-center rounded-md transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-400 disabled:cursor-not-allowed disabled:opacity-50",
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      {...props}
    >
      <Icon className={size === "sm" ? "size-3.5" : "size-4"} aria-hidden="true" />
    </button>
  );

  if (!tooltip) {
    return button;
  }

  if (tooltipSide) {
    return (
      <Tooltip content={tooltip} side={tooltipSide}>
        {button}
      </Tooltip>
    );
  }

  return <Tooltip content={tooltip}>{button}</Tooltip>;
}
