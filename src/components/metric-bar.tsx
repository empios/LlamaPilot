import { cn } from "@/lib/utils";

interface MetricBarProps {
  label: string;
  used: number;
  total: number;
  formatValue: (value: number) => string;
  sublabel?: string;
  className?: string;
}

/**
 * Labelled usage bar showing used, total, and the percentage.
 *
 * The percentage is spelled out because a bar alone forces the reader to estimate.
 */
export function MetricBar({
  label,
  used,
  total,
  formatValue,
  sublabel,
  className,
}: MetricBarProps) {
  const percentage = total > 0 ? Math.min(100, Math.round((used / total) * 100)) : 0;

  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <div className="flex items-baseline justify-between gap-4">
        <span className="truncate text-sm font-medium">{label}</span>
        <span className="shrink-0 font-mono text-xs tabular-nums text-muted-foreground">
          {formatValue(used)} / {formatValue(total)}
          <span className="ml-2 text-foreground">{percentage}%</span>
        </span>
      </div>
      <div
        role="meter"
        aria-label={`${label} memory in use`}
        aria-valuenow={percentage}
        aria-valuemin={0}
        aria-valuemax={100}
        className="h-1.5 w-full overflow-hidden rounded-full bg-muted"
      >
        <div
          className="h-full rounded-full bg-primary transition-[width] duration-300"
          style={{ width: `${percentage}%` }}
        />
      </div>
      {sublabel ? (
        <span className="text-xs text-muted-foreground">{sublabel}</span>
      ) : null}
    </div>
  );
}
