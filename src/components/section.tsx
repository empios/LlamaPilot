import type { LucideIcon } from "lucide-react";
import type { ComponentProps, ReactNode } from "react";

import { cn } from "@/lib/utils";

interface SectionProps extends ComponentProps<"section"> {
  /** Short uppercase eyebrow that names the section. */
  label: string;
  title?: string;
  description?: string;
  icon?: LucideIcon;
  actions?: ReactNode;
  bodyClassName?: string;
}

/**
 * Panel with an always-present labelled header.
 *
 * Used instead of a bare Card so every block of content states what it is, and so the header
 * rule gives the page visible structure rather than a run of identical rounded boxes.
 */
export function Section({
  label,
  title,
  description,
  icon: Icon,
  actions,
  children,
  className,
  bodyClassName,
  ...props
}: SectionProps) {
  return (
    <section
      className={cn(
        "overflow-hidden rounded-lg border border-border bg-card shadow-[var(--pg-shadow-01)]",
        className,
      )}
      {...props}
    >
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-border bg-muted/35 px-[18px] py-3">
        <div className="flex min-w-0 items-center gap-2.5">
          {Icon ? (
            <span className="flex size-7 shrink-0 items-center justify-center rounded-md bg-card text-[var(--pg-orange-60)] shadow-[var(--pg-shadow-01)] dark:text-[var(--pg-orange-30)]">
              <Icon className="size-3.5" />
            </span>
          ) : null}
          <div className="flex min-w-0 flex-col">
            <span className="text-[10px] font-semibold tracking-[0.08em] text-muted-foreground uppercase">
              {label}
            </span>
            {title ? (
              <span className="truncate text-sm font-semibold">{title}</span>
            ) : null}
            {description ? (
              <span className="truncate text-xs text-muted-foreground">{description}</span>
            ) : null}
          </div>
        </div>
        {actions ? <div className="flex items-center gap-2">{actions}</div> : null}
      </header>
      <div className={cn("p-[18px]", bodyClassName)}>{children}</div>
    </section>
  );
}
