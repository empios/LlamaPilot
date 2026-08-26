import { describe, expect, it } from "vitest";

import type { LlamaOption } from "@/types/capabilities";

import {
  orderedKnownOptions,
  profileOptionKey,
  settingForOption,
  updateProfileOption,
} from "./profile-option-controls";

function option(overrides: Partial<LlamaOption> = {}): LlamaOption {
  return {
    flag: "--ctx-size",
    aliases: ["-c", "--ctx-size"],
    valueHint: "N|auto",
    description: "context size",
    section: "common",
    category: "context",
    knownKey: "contextSize",
    displayName: "Context size",
    summary: "Maximum context size",
    ...overrides,
  };
}

describe("profile option storage", () => {
  it("reads values stored under canonical keys, flags, or aliases", () => {
    const context = option();

    expect(
      settingForOption({ contextSize: { mode: "auto" } }, context, "contextSize"),
    ).toEqual({ mode: "auto" });
    expect(
      settingForOption(
        { "-c": { mode: "custom", value: "32768" } },
        context,
        "contextSize",
      ),
    ).toEqual({ mode: "custom", value: "32768" });
  });

  it("migrates old aliases to one canonical key when changed", () => {
    const context = option();
    const stored = {
      "-c": { mode: "custom", value: "8192" } as const,
      "--ctx-size": { mode: "custom", value: "16384" } as const,
      unrelated: { mode: "auto" } as const,
    };

    expect(
      updateProfileOption(stored, context, "contextSize", {
        mode: "custom",
        value: "32768",
      }),
    ).toEqual({
      contextSize: { mode: "custom", value: "32768" },
      unrelated: { mode: "auto" },
    });
    expect(updateProfileOption(stored, context, "contextSize", { mode: "default" })).toEqual({
      unrelated: { mode: "auto" },
    });
  });

  it("uses a known key only when the runtime maps it unambiguously", () => {
    const context = option();

    expect(profileOptionKey(context, new Map([["contextSize", 1]]))).toBe("contextSize");
    expect(profileOptionKey(context, new Map([["contextSize", 2]]))).toBe("--ctx-size");
  });

  it("orders specialized controls by their product-defined key order", () => {
    const flash = option({ flag: "--flash-attn", aliases: ["--flash-attn"], knownKey: "flashAttention" });
    const layers = option({ flag: "--gpu-layers", aliases: ["--gpu-layers"], knownKey: "gpuLayers" });

    expect(orderedKnownOptions([flash, layers], ["gpuLayers", "flashAttention"])).toEqual([
      layers,
      flash,
    ]);
  });
});
