import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { UpdateStatus } from "@/types/updater";

const mocks = vi.hoisted(() => ({
  status: null as UpdateStatus | null,
  updates: { autoCheck: true, autoDownload: false, autoInstall: true },
  action: vi.fn(),
}));
vi.mock("@/hooks/use-app-update", () => ({
  useAppUpdate: () => ({ data: mocks.status }),
  useUpdateActions: () => mocks.action,
}));
vi.mock("@/hooks/use-settings", () => ({ useSettings: () => ({ data: { updates: mocks.updates } }) }));

import { UpdateBridge } from "./update-bridge";
import { useUpdateStore } from "@/stores/update-store";

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-09-14T12:00:00Z"));
  Object.defineProperty(document, "visibilityState", { configurable: true, value: "visible" });
  mocks.action.mockReset();
  mocks.updates = { autoCheck: true, autoDownload: false, autoInstall: true };
  mocks.status = {
    phase: "ready", currentVersion: "0.3.0", version: "0.4.0", notes: null,
    lastChecked: new Date().toISOString(), downloaded: 100, total: 100,
    canCheck: true, canInstall: true, reason: null, error: null,
    deferredVersion: null, busy: false, releasesUrl: "https://github.com/empios/LlamaPilot/releases/latest",
  };
  useUpdateStore.setState({ working: false, installing: false, countdown: null, error: null, blockers: {} });
});
afterEach(() => { cleanup(); vi.useRealTimers(); });

it("offers a visible countdown before automatic installation", () => {
  render(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(1000));
  expect(screen.getByText(/restart in 30s/)).toBeTruthy();
  expect(mocks.action).not.toHaveBeenCalled();
  act(() => vi.advanceTimersByTime(30_000));
  expect(mocks.action).toHaveBeenCalledWith("install", true);
});

it("resets the countdown when a server starts and waits another full countdown", () => {
  const view = render(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(20_000));
  mocks.status = { ...mocks.status!, busy: true };
  view.rerender(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(20_000));
  expect(mocks.action).not.toHaveBeenCalled();
  expect(useUpdateStore.getState().countdown).toBeNull();
  mocks.status = { ...mocks.status!, busy: false };
  view.rerender(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(30_000));
  expect(mocks.action).not.toHaveBeenCalled();
  act(() => vi.advanceTimersByTime(1000));
  expect(mocks.action).toHaveBeenCalledWith("install", true);
});

it("waits for dirty settings, dialogs and a visible window", () => {
  const view = render(<UpdateBridge />);
  act(() => useUpdateStore.setState({ blockers: { settings: true } }));
  act(() => vi.advanceTimersByTime(60_000));
  expect(mocks.action).not.toHaveBeenCalled();
  act(() => useUpdateStore.setState({ blockers: {} }));
  const dialog = document.createElement("div");
  dialog.setAttribute("role", "dialog");
  document.body.append(dialog);
  act(() => vi.advanceTimersByTime(60_000));
  expect(mocks.action).not.toHaveBeenCalled();
  dialog.remove();
  Object.defineProperty(document, "visibilityState", { configurable: true, value: "hidden" });
  view.rerender(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(60_000));
  expect(mocks.action).not.toHaveBeenCalled();
});

it("Later defers the version and disabled automation never installs", () => {
  const view = render(<UpdateBridge />);
  fireEvent.click(screen.getByRole("button", { name: "Later" }));
  expect(mocks.action).toHaveBeenCalledWith("defer");
  mocks.action.mockClear();
  mocks.status = { ...mocks.status!, deferredVersion: "0.4.0" };
  view.rerender(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(60_000));
  expect(mocks.action).not.toHaveBeenCalled();
  mocks.status = { ...mocks.status!, deferredVersion: null };
  mocks.updates.autoInstall = false;
  view.rerender(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(60_000));
  expect(mocks.action).not.toHaveBeenCalled();
});

it("checks once at startup and limits automatic download retries", () => {
  mocks.status = { ...mocks.status!, phase: "idle", lastChecked: null, version: null };
  const view = render(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(60_000));
  expect(mocks.action).toHaveBeenCalledTimes(1);
  expect(mocks.action).toHaveBeenCalledWith("check");
  mocks.action.mockClear();
  mocks.status = { ...mocks.status!, phase: "available", version: "0.4.0" };
  view.rerender(<UpdateBridge />);
  act(() => vi.advanceTimersByTime(60_000));
  expect(mocks.action).toHaveBeenCalledTimes(1);
  expect(mocks.action).toHaveBeenCalledWith("download");
});
