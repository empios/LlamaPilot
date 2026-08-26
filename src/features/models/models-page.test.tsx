import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const hooks = vi.hoisted(() => ({
  useModels: vi.fn(),
  useSetModelProjector: vi.fn(),
  useGgufPicker: vi.fn(),
  refetch: vi.fn(),
  mutateProjector: vi.fn(),
}));

vi.mock("@/hooks/use-models", () => ({
  useModels: hooks.useModels,
  useSetModelProjector: hooks.useSetModelProjector,
}));
vi.mock("@/hooks/use-gguf-picker", () => ({ useGgufPicker: hooks.useGgufPicker }));

import { useNavigationStore } from "@/stores/navigation-store";
import type { ModelCatalog } from "@/types/models";

import { ModelsPage } from "./models-page";

function catalog(overrides: Partial<ModelCatalog> = {}): ModelCatalog {
  return {
    scannedAt: "2026-08-26T10:00:00.000Z",
    roots: [],
    models: [],
    projectors: [],
    issues: [],
    cacheHits: 0,
    cacheMisses: 0,
    ...overrides,
  };
}

describe("models page", () => {
  beforeEach(() => {
    act(() => useNavigationStore.setState({ page: "models", selectedSourceId: null }));
    hooks.refetch.mockReset();
    hooks.mutateProjector.mockReset();
    hooks.useModels.mockReturnValue({
      data: catalog(),
      isPending: false,
      isError: false,
      isFetching: false,
      error: null,
      refetch: hooks.refetch,
    });
    hooks.useSetModelProjector.mockReturnValue({
      mutate: hooks.mutateProjector,
      isPending: false,
    });
    hooks.useGgufPicker.mockReturnValue(vi.fn());
  });

  afterEach(cleanup);

  it("guides a new user to configure model folders", () => {
    render(<ModelsPage />);

    expect(screen.getByText("No model folders configured")).toBeTruthy();
    expect(screen.getByText("Add a folder containing GGUF files")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Configure model folders" }));
    expect(useNavigationStore.getState().page).toBe("settings");
  });

  it("reports an empty configured catalog and allows an immediate rescan", () => {
    hooks.useModels.mockReturnValue({
      data: catalog({
        roots: ["E:\\models"],
        issues: [{ path: "E:\\models\\broken.gguf", message: "Invalid GGUF header" }],
        cacheMisses: 1,
      }),
      isPending: false,
      isError: false,
      isFetching: false,
      error: null,
      refetch: hooks.refetch,
    });

    render(<ModelsPage />);

    expect(screen.getByText("No primary model GGUF files found")).toBeTruthy();
    expect(screen.getByText("Invalid GGUF header")).toBeTruthy();
    expect(screen.getByText("0 logical models")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Scan now" }));
    expect(hooks.refetch).toHaveBeenCalledTimes(1);
  });
});
