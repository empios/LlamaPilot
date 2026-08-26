import { describe, expect, it } from "vitest";

import type { LlamaCapabilities, LlamaOption } from "@/types/capabilities";

import { pruneInactiveSpeculativeOptions } from "./profile-specialized-controls";

function option(flag: string, knownKey: string): LlamaOption {
  return {
    flag,
    aliases: [flag],
    valueHint: null,
    description: "",
    section: "speculative",
    category: "speculative",
    knownKey,
    displayName: knownKey,
    summary: null,
  };
}

const capabilities: LlamaCapabilities = {
  schemaVersion: 1,
  version: "test",
  commit: null,
  speculativeTypes: ["draft-simple", "ngram-simple"],
  devices: [],
  options: {
    speculativeType: option("--spec-type", "speculativeType"),
    draftModel: option("--model-draft", "draftModel"),
    draftMinimum: option("--draft-min", "specDraftMinTokens"),
    ngramSize: option("--lookup-ngram-min", "ngramSimpleSizeN"),
  },
};

describe("specialized profile controls", () => {
  it("removes settings from inactive speculative strategies", () => {
    const input = {
      speculativeType: { mode: "custom", value: "ngram-simple" } as const,
      "--model-draft": { mode: "custom", value: "draft.gguf" } as const,
      "--draft-min": { mode: "custom", value: "2" } as const,
      ngramSimpleSizeN: { mode: "custom", value: "4" } as const,
      contextSize: { mode: "custom", value: "32768" } as const,
    };

    expect(pruneInactiveSpeculativeOptions(input, capabilities)).toEqual({
      speculativeType: { mode: "custom", value: "ngram-simple" },
      ngramSimpleSizeN: { mode: "custom", value: "4" },
      contextSize: { mode: "custom", value: "32768" },
    });
    expect(input).toHaveProperty("--model-draft");
  });

  it("retains draft-model controls when an external strategy is selected", () => {
    const input = {
      "--spec-type": { mode: "custom", value: "draft-simple" } as const,
      "--model-draft": { mode: "custom", value: "draft.gguf" } as const,
      "--draft-min": { mode: "auto" } as const,
      ngramSimpleSizeN: { mode: "custom", value: "4" } as const,
    };

    expect(pruneInactiveSpeculativeOptions(input, capabilities)).toEqual({
      "--spec-type": { mode: "custom", value: "draft-simple" },
      "--model-draft": { mode: "custom", value: "draft.gguf" },
      "--draft-min": { mode: "auto" },
    });
  });
});
