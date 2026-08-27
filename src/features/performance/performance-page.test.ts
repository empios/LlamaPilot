import { describe, expect, it } from "vitest";

import { applyCandidateOptions } from "./performance-page";
import type { PerformanceCandidate } from "@/types/performance";
import type { LaunchProfile } from "@/types/profiles";

const profile: LaunchProfile = {
  schemaVersion: 1,
  id: "profile",
  name: "Coding",
  description: null,
  runtimeId: "runtime",
  runtimeLabel: "CUDA runtime",
  modelId: "model",
  modelName: "Coder",
  modelPath: "C:\\models\\coder.gguf",
  projectorPath: null,
  host: "127.0.0.1",
  port: 8080,
  autoSelectPort: false,
  options: {
    contextSize: { mode: "custom", value: "32768" },
    splitMode: { mode: "custom", value: "none" },
    kvCacheTypeK: { mode: "custom", value: "f16" },
  },
  environment: {},
  additionalArguments: [],
  createdAt: "2026-08-27T00:00:00Z",
  updatedAt: "2026-08-27T00:00:00Z",
};

const candidate: PerformanceCandidate = {
  id: "layer",
  label: "Layer",
  description: "All GPUs",
  splitMode: "layer",
  experimental: false,
  options: {
    device: { mode: "custom", value: "CUDA0,CUDA1" },
    splitMode: { mode: "custom", value: "layer" },
    tensorSplit: { mode: "custom", value: "50,50" },
    mainGpu: { mode: "default" },
  },
  reasons: [],
  warnings: [],
};

describe("Performance Lab candidate application", () => {
  it("changes placement while preserving unrelated profile settings", () => {
    const input = applyCandidateOptions(profile, candidate);

    expect(input.options.contextSize).toEqual({ mode: "custom", value: "32768" });
    expect(input.options.device).toEqual({ mode: "custom", value: "CUDA0,CUDA1" });
    expect(input.options.splitMode).toEqual({ mode: "custom", value: "layer" });
    expect(input.options.mainGpu).toBeUndefined();
    expect(input.options.kvCacheTypeK).toBeUndefined();
    expect(input.modelId).toBe("model");
  });
});
