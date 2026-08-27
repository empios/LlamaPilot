import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  start: vi.fn(),
  test: vi.fn(),
  reset: vi.fn(),
  serverState: "ready",
}));

const profile = {
  schemaVersion: 1,
  id: "profile-1",
  name: "Coding CUDA",
  description: null,
  runtimeId: "runtime-1",
  runtimeLabel: "CUDA · all GPUs",
  modelId: "model-1",
  modelName: "Coder GGUF",
  modelPath: "E:\\models\\coder.gguf",
  projectorPath: null,
  host: "0.0.0.0",
  port: 8080,
  autoSelectPort: true,
  options: { modelAlias: { mode: "custom", value: "coder" } },
  environment: {},
  additionalArguments: [],
  createdAt: "now",
  updatedAt: "now",
} as const;

vi.mock("@/hooks/use-profiles", () => ({
  useProfiles: () => ({
    data: [profile],
    isPending: false,
    error: null,
  }),
}));

vi.mock("@/hooks/use-server", () => ({
  useServerStatus: () => ({
    data: {
      generation: 1,
      state: mocks.serverState,
      profileId: mocks.serverState === "stopped" ? null : profile.id,
      host: mocks.serverState === "stopped" ? null : "0.0.0.0",
      port: mocks.serverState === "stopped" ? null : 49152,
    },
    isPending: false,
    error: null,
  }),
  useStartServer: () => ({ mutate: mocks.start, isPending: false }),
}));

vi.mock("@/hooks/use-agent", () => ({
  useTestAgentConnection: () => ({
    mutate: mocks.test,
    reset: mocks.reset,
    isPending: false,
    isError: false,
    error: null,
  }),
}));

import { AgentConnectPage } from "./agent-connect-page";

describe("Agent Connect page", () => {
  beforeEach(() => {
    mocks.start.mockReset();
    mocks.test.mockReset();
    mocks.reset.mockReset();
    mocks.serverState = "ready";
  });

  afterEach(cleanup);

  it("uses the effective server port and verifies models plus chat", () => {
    mocks.test.mockImplementation(
      (_request: unknown, options: { onSuccess: (result: unknown) => void }) => {
        options.onSuccess({
          apiBaseUrl: "http://127.0.0.1:49152/v1",
          modelIds: ["coder"],
          selectedModel: "coder",
          requestedModelMatched: true,
          responseText: "connected",
          modelsLatencyMs: 8,
          chatLatencyMs: 240,
        });
      },
    );
    render(<AgentConnectPage />);

    expect(screen.getByText("http://127.0.0.1:49152/v1")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Test connection" }));

    expect(mocks.test).toHaveBeenCalledWith(
      { apiKey: null, model: "coder" },
      expect.any(Object),
    );
    expect(screen.getByText("Agent connection verified")).toBeTruthy();
    expect(screen.getByText("connected")).toBeTruthy();

    expect(screen.getByRole("tab", { name: "OpenCode" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Pi" })).toBeTruthy();
  });

  it("starts the selected profile when the server is stopped", () => {
    mocks.serverState = "stopped";
    render(<AgentConnectPage />);

    expect(screen.getByText("http://127.0.0.1:8080/v1")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Start profile" }));
    expect(mocks.start).toHaveBeenCalledWith(profile.id);
  });
});
