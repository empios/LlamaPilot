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
      className="flex w-60 shrink-0 flex-col bg-sidebar text-sidebar-foreground"
    >
      <div className="flex h-16 items-center gap-2.5 px-5">
        <span className="flex size-7 items-center justify-center rounded-md bg-white/10 text-[var(--pg-orange-30)] ring-1 ring-white/10">
          <CpuIcon className="size-4" />
        </span>
        <div className="flex min-w-0 flex-col leading-tight">
          <span className="truncate text-[15px] font-bold tracking-tight text-white">
            {branding.name}
          </span>
          <span className="truncate text-[11px] text-white/50">
            {branding.tagline}
          </span>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-3.5 pb-4">
        {navigationGroups.map((group) => (
          <section key={group.label} className="flex flex-col gap-1">
            <h2 className="px-2.5 pt-3 pb-1 text-[10px] font-semibold tracking-[0.1em] text-white/45 uppercase">
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
                        "flex w-full items-center gap-2.5 rounded-md px-2.5 py-[7px] text-[13px]",
                        "transition-colors duration-100",
                        "focus-visible:ring-2 focus-visible:ring-sidebar-ring focus-visible:ring-offset-2 focus-visible:ring-offset-sidebar focus-visible:outline-none",
                        isActive
                          ? "bg-sidebar-primary font-medium text-white shadow-sm"
                          : "text-white/75 hover:bg-white/6 hover:text-white",
                      )}
                    >
                      <Icon className="size-4 shrink-0 stroke-[1.7]" />
                      <span className="truncate">{item.label}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </div>

      <footer className="mx-3.5 border-t border-white/10 px-2.5 py-3">
        <p className="text-[10px] font-semibold tracking-[0.1em] text-white/40 uppercase">
          Version
        </p>
        <p className="font-mono text-xs text-white/65">
          {appInfo.data ? appInfo.data.version : "—"}
        </p>
      </footer>
    </nav>
  );
}
