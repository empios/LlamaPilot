import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface StatTileProps {
  label: string;
  value: ReactNode;
  detail?: ReactNode;
  icon?: LucideIcon;
  /** Renders the value in a monospace face, for hashes, versions, and sizes. */
  mono?: boolean;
  className?: string;
}

/**
 * Labelled key/value readout.
 *
 * Values wrap to two lines rather than truncating, because a clipped CPU or GPU name is worse
 * than a slightly taller tile.
 */
export function StatTile({
  label,
  value,
  detail,
  icon: Icon,
  mono,
  className,
}: StatTileProps) {
  return (
    <div className={cn("flex min-w-0 flex-col gap-1", className)}>
      <div className="flex items-center gap-1.5 text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
        {Icon ? <Icon className="size-3.5 shrink-0" /> : null}
        <span className="truncate">{label}</span>
      </div>
      <div
        className={cn(
          "line-clamp-2 text-base leading-snug font-semibold text-balance",
          mono && "font-mono text-sm",
        )}
      >
        {value}
      </div>
      {detail ? (
        <div className="line-clamp-2 text-xs leading-snug text-muted-foreground">
          {detail}
        </div>
      ) : null}
    </div>
  );
}

/** Evenly spaced tiles separated by rules, which reads as a spec sheet rather than a card grid. */
export function StatGrid({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "grid items-start gap-x-6 gap-y-5 sm:grid-cols-2 lg:grid-cols-4",
        "lg:divide-x lg:divide-border lg:*:not-first:pl-6",
        className,
      )}
    >
      {children}
    </div>
  );
}
