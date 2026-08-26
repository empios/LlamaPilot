import { AppSidebar } from "@/app/app-sidebar";
import { AppTopbar } from "@/app/app-topbar";
import { BuildPage } from "@/features/builds/build-page";
import { DashboardPage } from "@/features/dashboard/dashboard-page";
import { LogsPage } from "@/features/logs/logs-page";
import { ModelsPage } from "@/features/models/models-page";
import { ProfilesPage } from "@/features/profiles/profiles-page";
import { RuntimesPage } from "@/features/runtime/runtimes-page";
import { SettingsPage } from "@/features/settings/settings-page";
import { useThemeSync } from "@/hooks/use-theme-sync";
import { useServerEventBridge } from "@/hooks/use-server";
import { useNavigationStore, type PageId } from "@/stores/navigation-store";

export function AppShell() {
  useThemeSync();
  useServerEventBridge();
  const page = useNavigationStore((state) => state.page);

  return (
    // Sized from the document rather than the viewport: `100vw`/`100vh` ignore scrollbar
    // gutters, which is how the shell ended up wider than the window it lives in.
    <div className="flex h-full w-full overflow-hidden bg-background text-foreground">
      <AppSidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <AppTopbar />
        <main className="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto">
          <div className="mx-auto w-full max-w-[1400px] px-8 py-7">{renderPage(page)}</div>
        </main>
      </div>
    </div>
  );
}

function renderPage(page: PageId) {
  switch (page) {
    case "dashboard":
      return <DashboardPage />;
    case "models":
      return <ModelsPage />;
    case "profiles":
      return <ProfilesPage />;
    case "runtimes":
      return <RuntimesPage />;
    case "build":
      return <BuildPage />;
    case "logs":
      return <LogsPage />;
    case "settings":
      return <SettingsPage />;
    default: {
      const exhaustive: never = page;
      return exhaustive;
    }
  }
}
