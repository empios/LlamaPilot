import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";

export type StatusTone = "neutral" | "success" | "warning" | "danger";

const toneStyles: Record<StatusTone, string> = {
  neutral: "text-muted-foreground",
  success: "text-success",
  warning: "text-warning",
  danger: "text-destructive",
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
        "inline-flex items-center gap-1.5 rounded-md border border-border bg-card px-2 py-1",
        className,
      )}
    >
      <span className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
        {label}
      </span>
      <span className={cn("flex items-center gap-1 font-mono text-xs", toneStyles[tone])}>
        {Icon ? <Icon className="size-3" /> : null}
        {value}
      </span>
    </span>
  );
}
