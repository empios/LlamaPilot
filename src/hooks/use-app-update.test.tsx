import { act, cleanup, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ install: vi.fn(), check: vi.fn(), download: vi.fn(), defer: vi.fn() }));
vi.mock("@/lib/ipc", () => ({ ipc: { installAppUpdate: mocks.install, checkAppUpdate: mocks.check, downloadAppUpdate: mocks.download, deferAppUpdate: mocks.defer } }));
import { useUpdateActions } from "./use-app-update";
import { useUpdateStore } from "@/stores/update-store";
import { useUpdateBlocker } from "./use-update-blocker";

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>;
}
beforeEach(() => {
  vi.resetAllMocks();
  useUpdateStore.setState({ working: false, installing: false, countdown: null, error: null, blockers: {} });
});
afterEach(cleanup);

it("does not send install IPC while an editor owns an unsaved draft", async () => {
  const blocker = renderHook(() => useUpdateBlocker(true));
  const { result } = renderHook(useUpdateActions, { wrapper });
  await act(() => result.current("install"));
  expect(mocks.install).not.toHaveBeenCalled();
  blocker.unmount();
  await act(() => result.current("install"));
  expect(mocks.install).toHaveBeenCalledWith(false);
});

it("freezes editing during installation and recovers after installer failure", async () => {
  let reject!: (reason: unknown) => void;
  mocks.install.mockImplementation(() => new Promise((_, fail) => { reject = fail; }));
  const { result } = renderHook(useUpdateActions, { wrapper });
  let installing!: Promise<void>;
  act(() => { installing = result.current("install", true); });
  expect(useUpdateStore.getState().installing).toBe(true);
  await act(async () => {
    reject({ code: "unsupported", message: "Server is running" });
    await installing;
  });
  expect(useUpdateStore.getState().installing).toBe(false);
  expect(useUpdateStore.getState().error).toBe("Server is running");
});

it("shows an offline error and allows a manual retry", async () => {
  mocks.check.mockRejectedValueOnce(new Error("Offline")).mockResolvedValueOnce(undefined);
  const { result } = renderHook(useUpdateActions, { wrapper });
  await act(() => result.current("check"));
  expect(useUpdateStore.getState().error).toBe("Offline");
  await act(() => result.current("check"));
  expect(useUpdateStore.getState().error).toBeNull();
  expect(mocks.check).toHaveBeenCalledTimes(2);
});

it("automatically defers a raced work admission without latching an error", async () => {
  mocks.install.mockRejectedValue({ code: "updateBusy", message: "A build just started" });
  const { result } = renderHook(useUpdateActions, { wrapper });
  await act(() => result.current("install", true));
  expect(useUpdateStore.getState().error).toBeNull();
  expect(useUpdateStore.getState().working).toBe(false);
  expect(useUpdateStore.getState().countdown).toBeNull();
  await act(() => result.current("install", false));
  expect(useUpdateStore.getState().error).toBe("A build just started");
});
