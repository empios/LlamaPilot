import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const hooks = vi.hoisted(() => ({
  useSources: vi.fn(),
  useModels: vi.fn(),
  useRuntimes: vi.fn(),
  useProfiles: vi.fn(),
  useServerStatus: vi.fn(),
  useStartServer: vi.fn(),
  useStopServer: vi.fn(),
  useRestartServer: vi.fn(),
  start: vi.fn(),
  stop: vi.fn(),
  restart: vi.fn(),
}));

vi.mock("@/hooks/use-sources", () => ({ useSources: hooks.useSources }));
vi.mock("@/hooks/use-models", () => ({ useModels: hooks.useModels }));
vi.mock("@/hooks/use-build", () => ({ useRuntimes: hooks.useRuntimes }));
vi.mock("@/hooks/use-profiles", () => ({ useProfiles: hooks.useProfiles }));
vi.mock("@/hooks/use-server", () => ({
  useServerStatus: hooks.useServerStatus,
  useStartServer: hooks.useStartServer,
  useStopServer: hooks.useStopServer,
  useRestartServer: hooks.useRestartServer,
}));

import { ServerPanel } from "./server-panel";
import { useNavigationStore } from "@/stores/navigation-store";
import type { ServerSnapshot } from "@/types/server";

const stopped: ServerSnapshot = {
  generation: 0,
  state: "stopped",
  pid: null,
  profileId: null,
  profileName: null,
  runtimeId: null,
  runtimeLabel: null,
  modelName: null,
  host: null,
  port: null,
  startedAt: null,
  stoppedAt: null,
  exitCode: null,
  healthStatus: null,
  healthMessage: null,
  lastError: null,
  logFile: null,
  telemetry: {
    propsAvailable: null,
    slotsAvailable: null,
    metricsAvailable: null,
    totalSlots: null,
    busySlots: null,
    requestsProcessing: null,
    requestsDeferred: null,
    promptTokensPerSecond: null,
    predictedTokensPerSecond: null,
    buildInfo: null,
  },
};

describe("dashboard server panel", () => {
  beforeEach(() => {
    act(() => useNavigationStore.setState({ page: "dashboard", selectedSourceId: null }));
    hooks.start.mockReset();
    hooks.stop.mockReset();
    hooks.restart.mockReset();
    hooks.useSources.mockReturnValue({ data: [] });
    hooks.useModels.mockReturnValue({ data: { models: [] } });
    hooks.useRuntimes.mockReturnValue({ data: [] });
    hooks.useProfiles.mockReturnValue({ data: [] });
    hooks.useServerStatus.mockReturnValue({ data: stopped, isPending: false });
    hooks.useStartServer.mockReturnValue({ mutate: hooks.start, isPending: false });
    hooks.useStopServer.mockReturnValue({ mutate: hooks.stop, isPending: false });
    hooks.useRestartServer.mockReturnValue({ mutate: hooks.restart, isPending: false });
  });

  afterEach(cleanup);

  it("guides a new user to register the first source", () => {
    render(<ServerPanel />);

    expect(screen.getByText("Add a source")).toBeTruthy();
    const start = screen.getByRole("button", { name: "Start server" }) as HTMLButtonElement;
    expect(start.disabled).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "Open Runtimes" }));
    expect(useNavigationStore.getState().page).toBe("runtimes");
  });

  it("starts the first saved profile when setup is complete", () => {
    hooks.useSources.mockReturnValue({ data: [{ id: "source-1" }] });
    hooks.useRuntimes.mockReturnValue({ data: [{ id: "runtime-1" }] });
    hooks.useModels.mockReturnValue({ data: { models: [{ complete: true }] } });
    hooks.useProfiles.mockReturnValue({ data: [{ id: "profile-1" }] });

    render(<ServerPanel />);

    expect(screen.getByText("Ready for agent")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Start server" }));
    expect(hooks.start).toHaveBeenCalledWith("profile-1");
  });

  it("shows live activity and active process controls", () => {
    hooks.useSources.mockReturnValue({ data: [{ id: "source-1" }] });
    hooks.useRuntimes.mockReturnValue({ data: [{ id: "runtime-1" }] });
    hooks.useModels.mockReturnValue({ data: { models: [{ complete: true }] } });
    hooks.useProfiles.mockReturnValue({ data: [{ id: "profile-1" }] });
    hooks.useServerStatus.mockReturnValue({
      data: {
        ...stopped,
        generation: 1,
        state: "ready",
        pid: 4242,
        profileId: "profile-1",
        profileName: "Local model",
        modelName: "Tiny model",
        host: "127.0.0.1",
        port: 8080,
        healthMessage: "Healthy.",
        telemetry: {
          ...stopped.telemetry,
          totalSlots: 1,
          busySlots: 0,
          predictedTokensPerSecond: 12.5,
        },
      },
      isPending: false,
    });

    render(<ServerPanel />);

    expect(screen.getByText("12.5 tok/s")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Restart" }));
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    expect(hooks.restart).toHaveBeenCalledTimes(1);
    expect(hooks.stop).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Open live logs" }));
    expect(useNavigationStore.getState().page).toBe("logs");
  });
});
