import { beforeEach, describe, expect, it } from "vitest";

import { useNavigationStore } from "./navigation-store";

describe("navigation store", () => {
  beforeEach(() => {
    useNavigationStore.setState({ page: "dashboard", selectedSourceId: null });
  });

  it("starts on the dashboard with no source selected", () => {
    const state = useNavigationStore.getState();

    expect(state.page).toBe("dashboard");
    expect(state.selectedSourceId).toBeNull();
  });

  it("navigating does not clear the selected source", () => {
    useNavigationStore.getState().selectSource("source-1");
    useNavigationStore.getState().navigate("runtimes");

    const state = useNavigationStore.getState();
    expect(state.page).toBe("runtimes");
    expect(state.selectedSourceId).toBe("source-1");
  });

  it("selecting null clears the source", () => {
    useNavigationStore.getState().selectSource("source-1");
    useNavigationStore.getState().selectSource(null);

    expect(useNavigationStore.getState().selectedSourceId).toBeNull();
  });
});
