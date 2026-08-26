import type { ReactNode } from "react";

interface PageHeaderProps {
  /** Short uppercase eyebrow naming the area, matching the sidebar group. */
  eyebrow: string;
  title: string;
  description?: string;
  actions?: ReactNode;
}

export function PageHeader({ eyebrow, title, description, actions }: PageHeaderProps) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-4">
      <div className="flex min-w-0 flex-col gap-1">
        <span className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
          {eyebrow}
        </span>
        <h1 className="text-2xl leading-tight font-semibold tracking-tight">{title}</h1>
        {description ? (
          <p className="max-w-2xl text-sm text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {actions ? <div className="flex items-center gap-2">{actions}</div> : null}
    </header>
  );
}
