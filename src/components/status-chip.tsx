import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";

export type StatusTone = "neutral" | "success" | "warning" | "danger";

const toneStyles: Record<StatusTone, string> = {
  neutral: "bg-muted text-muted-foreground",
  success: "bg-[var(--pg-support-success-bg)] text-success",
  warning: "bg-[var(--pg-support-warning-bg)] text-warning",
  danger: "bg-[var(--pg-support-error-bg)] text-destructive",
};

interface StatusChipProps {
  label: string;
  value: string;
  tone?: StatusTone;
  icon?: LucideIcon;
  className?: string;
}

/**
 * Compact always-labelled status readout.
 *
 * The label is rendered rather than hidden behind a tooltip so the meaning of a value like
 * "2.54.0" is never ambiguous.
 */
export function StatusChip({
  label,
  value,
  tone = "neutral",
  icon: Icon,
  className,
}: StatusChipProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-sm px-2 py-1",
        toneStyles[tone],
        className,
      )}
    >
      <span className="text-[10px] font-semibold tracking-[0.08em] opacity-70 uppercase">
        {label}
      </span>
      <span className="flex items-center gap-1 font-mono text-xs">
        {Icon ? <Icon className="size-3" /> : null}
        {value}
      </span>
    </span>
  );
}
