import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/app/app-sidebar", () => ({ AppSidebar: () => <aside>Sidebar</aside> }));
vi.mock("@/app/app-topbar", () => ({ AppTopbar: () => <header>Topbar</header> }));
vi.mock("@/hooks/use-theme-sync", () => ({ useThemeSync: vi.fn() }));
vi.mock("@/hooks/use-server", () => ({ useServerEventBridge: vi.fn() }));
vi.mock("@/features/settings/update-bridge", () => ({ UpdateBridge: () => null }));
vi.mock("@/features/dashboard/dashboard-page", () => ({
  DashboardPage: () => <div>Dashboard page</div>,
}));
vi.mock("@/features/models/models-page", () => ({ ModelsPage: () => <div>Models page</div> }));
vi.mock("@/features/profiles/profiles-page", () => ({
  ProfilesPage: () => <div>Profiles page</div>,
}));
vi.mock("@/features/performance/performance-page", () => ({
  PerformancePage: () => <div>Performance page</div>,
}));
vi.mock("@/features/agent/agent-connect-page", () => ({
  AgentConnectPage: () => <div>Agent Connect page</div>,
}));
vi.mock("@/features/runtime/runtimes-page", () => ({
  RuntimesPage: () => <div>Runtimes page</div>,
}));
vi.mock("@/features/builds/build-page", () => ({ BuildPage: () => <div>Build page</div> }));
vi.mock("@/features/logs/logs-page", () => ({ LogsPage: () => <div>Logs page</div> }));
vi.mock("@/features/settings/settings-page", () => ({
  SettingsPage: () => <div>Settings page</div>,
}));

import { AppShell } from "./app-shell";
import { useNavigationStore } from "@/stores/navigation-store";

describe("application shell", () => {
  beforeEach(() => {
    act(() => {
      useNavigationStore.setState({ page: "dashboard", selectedSourceId: null });
    });
  });

  it("loads the active page and preserves the shell while navigating", async () => {
    render(<AppShell />);

    expect(screen.getByText("Sidebar")).toBeTruthy();
    expect(screen.getByText("Topbar")).toBeTruthy();
    await screen.findByText("Dashboard page");

    act(() => useNavigationStore.getState().navigate("profiles"));

    await waitFor(() => expect(screen.getByText("Profiles page")).toBeTruthy());
    expect(screen.getByText("Sidebar")).toBeTruthy();
    expect(screen.getByText("Topbar")).toBeTruthy();

    act(() => useNavigationStore.getState().navigate("performance"));
    await waitFor(() => expect(screen.getByText("Performance page")).toBeTruthy());

    act(() => useNavigationStore.getState().navigate("agent"));
    await waitFor(() => expect(screen.getByText("Agent Connect page")).toBeTruthy());
  });
});
