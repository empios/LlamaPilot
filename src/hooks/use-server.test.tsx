import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const bridge = vi.hoisted(() => {
  const state: { onEvent?: (event: unknown) => void } = {};
  return {
    state,
    hasTauriRuntime: vi.fn(),
    createServerEventChannel: vi.fn((onEvent: (event: unknown) => void) => {
      state.onEvent = onEvent;
      return { onmessage: onEvent };
    }),
    ipc: {
      getServerStatus: vi.fn(),
      getServerLogs: vi.fn(),
      subscribeServerEvents: vi.fn(),
      unsubscribeServerEvents: vi.fn(),
      startServer: vi.fn(),
      stopServer: vi.fn(),
      restartServer: vi.fn(),
      clearServerLogs: vi.fn(),
    },
  };
});

vi.mock("@/lib/ipc", () => ({
  createServerEventChannel: bridge.createServerEventChannel,
  hasTauriRuntime: bridge.hasTauriRuntime,
  ipc: bridge.ipc,
}));

import { useServerEventBridge } from "./use-server";
import { queryKeys } from "@/lib/query-keys";
import type { ServerLogsSnapshot, ServerSnapshot } from "@/types/server";

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

const emptyLogs: ServerLogsSnapshot = {
  entries: [],
  droppedEntries: 0,
  nextSequence: 1,
  currentFile: "C:\\Logs\\server.log",
};

describe("server event bridge", () => {
  beforeEach(() => {
    bridge.state.onEvent = undefined;
    bridge.hasTauriRuntime.mockReset().mockReturnValue(true);
    bridge.createServerEventChannel.mockClear();
    bridge.ipc.getServerStatus.mockReset().mockResolvedValue(stopped);
    bridge.ipc.getServerLogs.mockReset().mockResolvedValue(emptyLogs);
    bridge.ipc.subscribeServerEvents.mockReset().mockResolvedValue("subscription-1");
    bridge.ipc.unsubscribeServerEvents.mockReset().mockResolvedValue(true);
  });

  it("keeps status and logs synchronized and unsubscribes on unmount", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: PropsWithChildren) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    const view = renderHook(() => useServerEventBridge(), { wrapper });

    await waitFor(() => {
      expect(bridge.ipc.subscribeServerEvents).toHaveBeenCalledTimes(1);
      expect(queryClient.getQueryData(queryKeys.serverLogs)).toEqual(emptyLogs);
    });

    const ready = {
      ...stopped,
      generation: 1,
      state: "ready" as const,
      pid: 1234,
      profileId: "profile-1",
      profileName: "Local model",
    };
    act(() => {
      bridge.state.onEvent?.({ kind: "status", snapshot: ready });
      bridge.state.onEvent?.({
        kind: "log",
        entry: {
          sequence: 1,
          timestamp: "2026-08-26T12:00:00Z",
          stream: "stdout",
          level: "info",
          fact: "listening",
          text: "server is listening",
        },
      });
    });

    expect(queryClient.getQueryData(queryKeys.serverStatus)).toEqual(ready);
    expect(queryClient.getQueryData<ServerLogsSnapshot>(queryKeys.serverLogs)).toMatchObject({
      nextSequence: 2,
      droppedEntries: 0,
      entries: [{ sequence: 1, text: "server is listening" }],
    });

    act(() => {
      bridge.state.onEvent?.({ kind: "logsCleared", nextSequence: 8 });
    });
    expect(queryClient.getQueryData(queryKeys.serverLogs)).toEqual({
      entries: [],
      droppedEntries: 0,
      nextSequence: 8,
      currentFile: "C:\\Logs\\server.log",
    });

    view.unmount();
    expect(bridge.ipc.unsubscribeServerEvents).toHaveBeenCalledWith("subscription-1");
  });

  it("does not subscribe when rendered outside Tauri", async () => {
    bridge.hasTauriRuntime.mockReturnValue(false);
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: PropsWithChildren) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    renderHook(() => useServerEventBridge(), { wrapper });

    await waitFor(() => expect(bridge.ipc.getServerStatus).toHaveBeenCalled());
    expect(bridge.ipc.subscribeServerEvents).not.toHaveBeenCalled();
  });
});
