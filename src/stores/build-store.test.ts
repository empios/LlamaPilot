import { beforeEach, describe, expect, it } from "vitest";

import { useBuildLogStore } from "@/stores/build-store";

describe("build log store", () => {
  beforeEach(() => {
    useBuildLogStore.getState().clear();
  });

  it("keeps streamed output outside the Build page component", () => {
    const log = useBuildLogStore.getState();
    log.append({ kind: "started", label: "Building llama-server" });
    log.append({ kind: "output", stream: "stderr", text: "[20%] Compiling" });

    expect(useBuildLogStore.getState().lines).toEqual([
      { stream: "stdout", text: "> Building llama-server" },
      { stream: "stderr", text: "[20%] Compiling" },
    ]);
  });

  it("a new operation replaces output from the previous build", () => {
    const log = useBuildLogStore.getState();
    log.append({ kind: "output", stream: "stdout", text: "old" });
    log.append({ kind: "started", label: "New build" });

    expect(useBuildLogStore.getState().lines).toEqual([
      { stream: "stdout", text: "> New build" },
    ]);
  });
});
