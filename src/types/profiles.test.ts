import { describe, expect, it } from "vitest";

import {
  commandPreviewSchema,
  createProfileInput,
  launchProfileSchema,
} from "@/types/profiles";

describe("launch profile schemas", () => {
  it("creates a default-safe input with no option overrides", () => {
    const input = createProfileInput({
      host: "127.0.0.1",
      port: 8080,
      autoSelectPort: true,
      runtimeId: "runtime-1",
      modelId: "model-1",
    });
    expect(input.options).toEqual({});
    expect(input.additionalArguments).toEqual([]);
  });

  it("accepts the persisted one-file profile shape", () => {
    const input = createProfileInput({
      host: "127.0.0.1",
      port: 8080,
      autoSelectPort: true,
      runtimeId: "runtime-1",
      modelId: "model-1",
    });
    const parsed = launchProfileSchema.parse({
      ...input,
      options: {
        contextSize: { mode: "custom", value: "8192" },
        gpuLayers: { mode: "auto" },
      },
      schemaVersion: 1,
      id: "profile-1",
      runtimeLabel: "master @ abc1234 · CUDA",
      modelName: "Tiny",
      modelPath: "E:\\models\\tiny.gguf",
      projectorPath: null,
      createdAt: "2026-08-25T20:00:00Z",
      updatedAt: "2026-08-25T20:00:00Z",
    });
    expect(parsed.options.contextSize).toEqual({ mode: "custom", value: "8192" });
  });

  it("keeps the executable, discrete arguments, environment, and both renderings", () => {
    const parsed = commandPreviewSchema.parse({
      program: "E:\\runtime\\llama-server.exe",
      arguments: ["--model", "E:\\models\\tiny.gguf"],
      environment: { LLAMA_LOG_COLORS: "1" },
      plain: "llama-server.exe --model tiny.gguf",
      powershell: "& 'llama-server.exe' '--model' 'tiny.gguf'",
      runtimeLabel: "master @ abc1234 · CUDA",
      modelName: "Tiny",
      capabilityVersion: "b9000-abc1234",
      warnings: [],
    });
    expect(parsed.arguments).toHaveLength(2);
    expect(parsed.environment.LLAMA_LOG_COLORS).toBe("1");
  });
});
