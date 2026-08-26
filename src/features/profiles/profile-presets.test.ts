import { describe, expect, it } from "vitest";

import type { LlamaCapabilities, LlamaOption } from "@/types/capabilities";

import { applySingleUser131kPreset } from "./profile-presets";

function option(
  flag: string,
  knownKey: string | null,
  valueHint: string | null = "N",
): LlamaOption {
  return {
    flag,
    aliases: [flag],
    valueHint,
    description: "",
    section: "common params",
    category: knownKey ? "context" : "advanced",
    knownKey,
    displayName: flag,
    summary: null,
  };
}

const capabilities: LlamaCapabilities = {
  schemaVersion: 1,
  version: "test",
  commit: null,
  speculativeTypes: ["none", "draft-mtp"],
  devices: [],
  options: {
    "--ctx-size": option("--ctx-size", "contextSize"),
    "--parallel": option("--parallel", "parallel"),
    "--gpu-layers": option("--gpu-layers", "gpuLayers"),
    "--reasoning": option("--reasoning", "reasoning", "[on|off|auto]"),
    "--reasoning-budget": option("--reasoning-budget", "reasoning"),
    "--spec-draft-n-max": option("--spec-draft-n-max", "specDraftMaxTokens"),
  },
};

describe("single-user 131K profile preset", () => {
  it("sets supported throughput options without removing the selected drafter", () => {
    const result = applySingleUser131kPreset(
      {
        draftModel: { mode: "custom", value: "E:\\models\\mtp.gguf" },
        speculativeType: { mode: "custom", value: "draft-mtp" },
      },
      capabilities,
    );

    expect(result.options.contextSize).toEqual({ mode: "custom", value: "131072" });
    expect(result.options.parallel).toEqual({ mode: "custom", value: "1" });
    expect(result.options.gpuLayers).toEqual({ mode: "custom", value: "all" });
    expect(result.options["--reasoning"]).toEqual({ mode: "custom", value: "on" });
    expect(result.options["--reasoning-budget"]).toEqual({ mode: "custom", value: "8192" });
    expect(result.options.specDraftMaxTokens).toEqual({ mode: "custom", value: "2" });
    expect(result.options.draftModel).toEqual({ mode: "custom", value: "E:\\models\\mtp.gguf" });
  });

  it("does not enable draft-only tuning without a draft strategy", () => {
    const result = applySingleUser131kPreset({}, capabilities);
    expect(result.options.specDraftMaxTokens).toBeUndefined();
  });
});
