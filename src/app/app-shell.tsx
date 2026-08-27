import { lazy, Suspense } from "react";

import { AppSidebar } from "@/app/app-sidebar";
import { AppTopbar } from "@/app/app-topbar";
import { Skeleton } from "@/components/ui/skeleton";
import { useThemeSync } from "@/hooks/use-theme-sync";
import { useServerEventBridge } from "@/hooks/use-server";
import { useNavigationStore, type PageId } from "@/stores/navigation-store";

const DashboardPage = lazy(() =>
  import("@/features/dashboard/dashboard-page").then((module) => ({
    default: module.DashboardPage,
  })),
);
const ModelsPage = lazy(() =>
  import("@/features/models/models-page").then((module) => ({ default: module.ModelsPage })),
);
const ProfilesPage = lazy(() =>
  import("@/features/profiles/profiles-page").then((module) => ({
    default: module.ProfilesPage,
  })),
);
const PerformancePage = lazy(() =>
  import("@/features/performance/performance-page").then((module) => ({
    default: module.PerformancePage,
  })),
);
const RuntimesPage = lazy(() =>
  import("@/features/runtime/runtimes-page").then((module) => ({
    default: module.RuntimesPage,
  })),
);
const BuildPage = lazy(() =>
  import("@/features/builds/build-page").then((module) => ({ default: module.BuildPage })),
);
const LogsPage = lazy(() =>
  import("@/features/logs/logs-page").then((module) => ({ default: module.LogsPage })),
);
const SettingsPage = lazy(() =>
  import("@/features/settings/settings-page").then((module) => ({
    default: module.SettingsPage,
  })),
);

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
          <div className="mx-auto w-full max-w-[1400px] px-8 py-7">
            <Suspense fallback={<PageLoadingFallback />}>{renderPage(page)}</Suspense>
          </div>
        </main>
      </div>
    </div>
  );
}

function PageLoadingFallback() {
  return (
    <div className="flex flex-col gap-6" aria-label="Loading page">
      <div className="flex flex-col gap-3">
        <Skeleton className="h-3 w-20" />
        <Skeleton className="h-9 w-64" />
        <Skeleton className="h-4 w-full max-w-xl" />
      </div>
      <Skeleton className="h-52 w-full" />
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
    case "performance":
      return <PerformancePage />;
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
