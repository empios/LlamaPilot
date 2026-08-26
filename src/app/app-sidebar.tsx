import { CpuIcon } from "lucide-react";

import { navigationGroups } from "@/app/navigation";
import { useAppInfo } from "@/hooks/use-app-info";
import { branding } from "@/lib/branding";
import { cn } from "@/lib/utils";
import { useNavigationStore } from "@/stores/navigation-store";

export function AppSidebar() {
  const page = useNavigationStore((state) => state.page);
  const navigate = useNavigationStore((state) => state.navigate);
  const appInfo = useAppInfo();

  return (
    <nav
      aria-label="Primary"
      className="flex w-60 shrink-0 flex-col border-r border-border bg-sidebar"
    >
      <div className="flex h-14 items-center gap-2.5 border-b border-border px-4">
        <span className="flex size-7 items-center justify-center rounded-md bg-primary text-primary-foreground">
          <CpuIcon className="size-4" />
        </span>
        <div className="flex min-w-0 flex-col leading-tight">
          <span className="truncate text-sm font-semibold">{branding.name}</span>
          <span className="truncate text-xs text-muted-foreground">
            {branding.tagline}
          </span>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-3 py-4">
        {navigationGroups.map((group) => (
          <section key={group.label} className="flex flex-col gap-1">
            <h2 className="px-2.5 pb-1 text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
              {group.label}
            </h2>
            <ul className="flex flex-col gap-0.5">
              {group.items.map((item) => {
                const isActive = item.id === page;
                const Icon = item.icon;

                return (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => navigate(item.id)}
                      aria-current={isActive ? "page" : undefined}
                      className={cn(
                        "relative flex w-full items-center gap-2.5 rounded-md py-2 pr-2.5 pl-3 text-sm",
                        "transition-colors duration-100",
                        "focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none",
                        isActive
                          ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground"
                          : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-accent-foreground",
                      )}
                    >
                      {isActive ? (
                        <span
                          aria-hidden
                          className="absolute top-1.5 bottom-1.5 left-0 w-0.5 rounded-full bg-primary"
                        />
                      ) : null}
                      <Icon className="size-4 shrink-0" />
                      <span className="truncate">{item.label}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </div>

      <footer className="border-t border-border px-4 py-3">
        <p className="text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
          Version
        </p>
        <p className="font-mono text-xs text-muted-foreground">
          {appInfo.data ? appInfo.data.version : "—"}
        </p>
      </footer>
    </nav>
  );
}
