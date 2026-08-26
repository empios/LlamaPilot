import { beforeEach, describe, expect, it, vi } from "vitest";

const tauri = vi.hoisted(() => {
  class MockChannel {
    onmessage?: (message: unknown) => void;
  }

  return {
    Channel: MockChannel,
    invoke: vi.fn(),
    isTauri: vi.fn(),
  };
});

vi.mock("@tauri-apps/api/core", () => tauri);

import {
  createProgressChannel,
  createServerEventChannel,
  hasTauriRuntime,
  ipc,
} from "./ipc";

describe("typed IPC boundary", () => {
  beforeEach(() => {
    tauri.invoke.mockReset();
    tauri.isTauri.mockReset();
  });

  it("invokes the named backend command and validates its response", async () => {
    tauri.invoke.mockResolvedValue("2.54.0.windows.1");

    await expect(ipc.getGitVersion()).resolves.toBe("2.54.0.windows.1");
    expect(tauri.invoke).toHaveBeenCalledWith("get_git_version", undefined);
  });

  it("keeps arguments structured and accepts a void backend response", async () => {
    tauri.invoke.mockResolvedValue(null);

    await expect(ipc.revealPath("C:\\Models\\model.gguf")).resolves.toBeUndefined();
    expect(tauri.invoke).toHaveBeenCalledWith("reveal_path", {
      path: "C:\\Models\\model.gguf",
    });
  });

  it("turns malformed backend payloads into an actionable internal error", async () => {
    tauri.invoke.mockResolvedValue(254);

    await expect(ipc.getGitVersion()).rejects.toMatchObject({
      code: "internal",
      message: 'The backend returned an unexpected shape for "get_git_version".',
      hint: "This usually means the Rust and frontend versions are out of sync.",
    });
  });

  it("preserves structured backend errors", async () => {
    const error = {
      code: "dirtyWorktree",
      message: "The repository has uncommitted changes.",
      hint: "Commit or stash the changes first.",
      details: "raw git status",
    };
    tauri.invoke.mockRejectedValue(error);

    await expect(ipc.updateSource("source-1")).rejects.toEqual(error);
  });

  it("normalizes unknown thrown values", async () => {
    tauri.invoke.mockRejectedValue(new Error("IPC disconnected"));

    await expect(ipc.getSettings()).rejects.toEqual({
      code: "internal",
      message: "IPC disconnected",
    });
  });
});

describe("IPC channels", () => {
  it("forwards progress events without rewriting them", () => {
    const onEvent = vi.fn();
    const channel = createProgressChannel(onEvent) as unknown as {
      onmessage?: (message: unknown) => void;
    };
    const event = { kind: "stdout", line: "building" };

    channel.onmessage?.(event);

    expect(onEvent).toHaveBeenCalledWith(event);
  });

  it("forwards only server events that match the frontend schema", () => {
    const onEvent = vi.fn();
    const channel = createServerEventChannel(onEvent) as unknown as {
      onmessage?: (message: unknown) => void;
    };

    channel.onmessage?.({ kind: "logsCleared", nextSequence: 42 });
    channel.onmessage?.({ kind: "futureEvent", value: "ignored" });

    expect(onEvent).toHaveBeenCalledTimes(1);
    expect(onEvent).toHaveBeenCalledWith({ kind: "logsCleared", nextSequence: 42 });
  });

  it("reports whether the Tauri runtime is available", () => {
    tauri.isTauri.mockReturnValue(true);
    expect(hasTauriRuntime()).toBe(true);

    tauri.isTauri.mockReturnValue(false);
    expect(hasTauriRuntime()).toBe(false);
  });
});
