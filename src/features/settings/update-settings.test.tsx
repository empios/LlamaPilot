import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { UpdateStatus } from "@/types/updater";
import { updateStatusSchema } from "@/types/updater";
import { settingsSchema } from "@/types/settings";
const mocks = vi.hoisted(() => ({ status: null as UpdateStatus | null, action: vi.fn(), save: vi.fn() }));
vi.mock("@/hooks/use-app-update", () => ({ useAppUpdate: () => ({ data: mocks.status }), useUpdateActions: () => mocks.action }));
vi.mock("@/hooks/use-settings", () => ({ useUpdateSettings: () => ({ mutate: mocks.save, isPending: false }) }));
import { UpdateSettings } from "./update-settings";
import { useUpdateStore } from "@/stores/update-store";

// An existing v0.3.0 settings document has no updates section.
const settings = settingsSchema.parse({
  schemaVersion: 1, appearance: { theme: "system", compactDensity: false },
  workspace: { sourcesDirectory: null, buildsDirectory: null, modelDirectories: [] },
  git: { defaultRepository: "https://github.com/ggml-org/llama.cpp", executable: null },
  build: { cmakeExecutable: null, parallelJobs: null },
  server: { defaultHost: "127.0.0.1", defaultPort: 8080, autoSelectPort: true },
});
beforeEach(() => {
  vi.clearAllMocks();
  useUpdateStore.setState({ working: false, installing: false, error: null });
  mocks.status = {
    phase: "available", currentVersion: "0.3.0", version: "0.4.0", notes: "Release notes",
    lastChecked: null, downloaded: 12, total: null, canCheck: true, canInstall: true,
    reason: null, error: null, deferredVersion: null, busy: false, releasesUrl: "https://github.com/empios/LlamaPilot/releases/latest",
  };
});
afterEach(cleanup);

it("shows a missing release feed without claiming the app is up to date and allows retry", () => {
  mocks.status = updateStatusSchema.parse({ ...mocks.status!, phase: "unavailable", version: null, notes: null, lastChecked: "2026-09-14T09:00:00Z" });
  render(<UpdateSettings settings={settings} />);
  expect(screen.getByRole("status").textContent).toContain("Update service unavailable");
  expect(screen.queryByText("You are up to date.")).toBeNull();
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.queryByRole("button", { name: "Download update" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Check now" }));
  expect(mocks.action).toHaveBeenCalledWith("check");
});

it("shows up to date only after a successful check with no newer version", () => {
  mocks.status = { ...mocks.status!, phase: "idle", version: null, notes: null, lastChecked: "2026-09-14T09:00:00Z" };
  render(<UpdateSettings settings={settings} />);
  expect(screen.getByRole("status").textContent).toBe("You are up to date.");
});

it("defaults old settings to checks only and saves an explicit auto-install choice", () => {
  render(<UpdateSettings settings={settings} />);
  expect(settings.updates).toEqual({ autoCheck: true, autoDownload: false, autoInstall: false });
  fireEvent.click(screen.getByRole("switch", { name: /Install automatically when idle/ }));
  expect(mocks.save).toHaveBeenCalledWith({ ...settings, updates: { ...settings.updates, autoInstall: true } });
});

it("shows unknown download size without inventing a percentage", () => {
  mocks.status!.phase = "downloading";
  render(<UpdateSettings settings={settings} />);
  expect(screen.getByText(/total size unknown/)).toBeTruthy();
  expect(screen.getByRole("progressbar").hasAttribute("value")).toBe(false);
});

it("offers manual packages for unsupported installations and disables installation during work", () => {
  mocks.status!.canInstall = false;
  mocks.status!.reason = "MSI requires a manual update.";
  const view = render(<UpdateSettings settings={settings} />);
  expect(screen.queryByRole("button", { name: "Download update" })).toBeNull();
  expect(screen.getByRole("button", { name: "Release downloads" })).toBeTruthy();
  mocks.status = { ...mocks.status!, canInstall: true, phase: "ready", busy: true };
  view.rerender(<UpdateSettings settings={settings} />);
  expect((screen.getByRole("button", { name: "Install and restart" }) as HTMLButtonElement).disabled).toBe(true);
});
