import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const hooks = vi.hoisted(() => ({
  useServerLogs: vi.fn(),
  useServerStatus: vi.fn(),
  useClearServerLogs: vi.fn(),
  clearLogs: vi.fn(),
}));

vi.mock("@/hooks/use-server", () => ({
  useServerLogs: hooks.useServerLogs,
  useServerStatus: hooks.useServerStatus,
  useClearServerLogs: hooks.useClearServerLogs,
}));

import type { ServerLogEntry, ServerSnapshot } from "@/types/server";

import { LogsPage } from "./logs-page";

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

function entry(sequence: number, text: string, level: ServerLogEntry["level"]): ServerLogEntry {
  return {
    sequence,
    timestamp: `2026-08-26T10:00:0${sequence}.000Z`,
    stream: level === "error" ? "stderr" : "stdout",
    level,
    fact: null,
    text,
  };
}

function setLogs(entries: ServerLogEntry[]) {
  hooks.useServerLogs.mockReturnValue({
    data: { entries, droppedEntries: 0, nextSequence: entries.length + 1, currentFile: null },
    error: null,
  });
}

describe("logs page", () => {
  beforeEach(() => {
    hooks.clearLogs.mockReset();
    setLogs([]);
    hooks.useServerStatus.mockReturnValue({ data: stopped, error: null });
    hooks.useClearServerLogs.mockReturnValue({
      mutate: hooks.clearLogs,
      isPending: false,
    });
  });

  afterEach(cleanup);

  it("explains how to start capturing output while the server is offline", () => {
    render(<LogsPage />);

    expect(screen.getByText("Server is offline")).toBeTruthy();
    expect(screen.getByText("Start a launch profile to stream llama-server output here.")).toBeTruthy();
    expect(
      screen.getByText((_, element) =>
        element?.tagName === "SPAN" && element.textContent === "Showing 0 of 0 lines",
      ),
    ).toBeTruthy();
  });

  it("filters output and keeps a stable snapshot while paused", () => {
    const initial = [
      entry(1, "model loaded successfully", "info"),
      entry(2, "fatal allocation failure", "error"),
    ];
    setLogs(initial);
    const { rerender } = render(<LogsPage />);

    fireEvent.change(screen.getByPlaceholderText("Search server output"), {
      target: { value: "fatal" },
    });
    expect(screen.queryByText("model loaded successfully")).toBeNull();
    expect(screen.getByText("fatal allocation failure")).toBeTruthy();
    expect(screen.getByText("Reset filters")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Clear search" }));
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));
    expect(screen.getByText("Output is paused at 2 lines. New entries are still being captured.")).toBeTruthy();

    setLogs([...initial, entry(3, "new live entry", "info")]);
    rerender(<LogsPage />);
    expect(screen.queryByText("new live entry")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Resume" }));
    expect(screen.getByText("new live entry")).toBeTruthy();
  });
});
