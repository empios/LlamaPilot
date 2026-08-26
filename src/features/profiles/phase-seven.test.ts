import { describe, expect, it } from "vitest";

import type { LlamaOption } from "@/types/capabilities";

import {
  optionChoices,
  selectedSpeculativeTypes,
  speculativeControlGroups,
} from "./phase-seven";

function option(valueHint: string | null, description = ""): LlamaOption {
  return {
    flag: "--test",
    aliases: ["--test"],
    valueHint,
    description,
    section: "common params",
    category: "advanced",
    knownKey: null,
    displayName: "Test",
    summary: null,
  };
}

describe("phase seven profile helpers", () => {
  it("reads runtime-advertised enum and cache choices", () => {
    expect(optionChoices(option("{none,layer,row,tensor}"))).toEqual([
      "none",
      "layer",
      "row",
      "tensor",
    ]);
    expect(
      optionChoices(option("TYPE", "KV cache\nallowed values: f32, f16, q8_0")),
    ).toEqual(["f32", "f16", "q8_0"]);
    expect(optionChoices(option("N0,N1,N2,..."))).toEqual([]);
  });

  it("normalizes speculative selections and removes disabled mode", () => {
    expect(selectedSpeculativeTypes({
      speculativeType: {
        mode: "custom",
        value: "none, ngram-mod,ngram-mod,draft-dflash",
      },
    })).toEqual(["ngram-mod", "draft-dflash"]);
  });

  it("renders only active strategy groups and never duplicates shared controls", () => {
    const groups = speculativeControlGroups([
      "draft-dflash",
      "ngram-simple",
      "ngram-mod",
    ]);
    expect(groups.map((group) => group.id)).toEqual([
      "draft-model",
      "draft-tuning",
      "ngram-simple",
      "ngram-mod",
    ]);
    expect(groups.flatMap((group) => group.keys).filter(
      (key) => key === "specDraftMaxTokens",
    )).toHaveLength(1);
  });

  it("treats MTP as an external draft-model strategy", () => {
    const groups = speculativeControlGroups(["draft-mtp"]);
    expect(groups.map((group) => group.id)).toEqual([
      "draft-model",
      "draft-tuning",
    ]);
    expect(groups[0]?.keys).toContain("draftModel");
  });
});
