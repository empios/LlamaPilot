import { describe, expect, it } from "vitest";

import type { LaunchProfile } from "@/types/profiles";

import {
  buildAgentSnippet,
  clientApiBaseUrl,
  profileModelIdentifiers,
} from "./connection-config";

function profile(options: LaunchProfile["options"] = {}): LaunchProfile {
  return {
    schemaVersion: 1,
    id: "profile",
    name: "Coding",
    description: null,
    runtimeId: "runtime",
    runtimeLabel: "CUDA",
    modelId: "model",
    modelName: "Coder",
    modelPath: "E:\\models\\coder.gguf",
    projectorPath: null,
    host: "0.0.0.0",
    port: 8080,
    autoSelectPort: false,
    options,
    environment: {},
    additionalArguments: [],
    createdAt: "now",
    updatedAt: "now",
  };
}

describe("Agent Connect configuration", () => {
  it("normalizes wildcard and IPv6 listener addresses for local clients", () => {
    expect(clientApiBaseUrl("0.0.0.0", 8080)).toBe("http://127.0.0.1:8080/v1");
    expect(clientApiBaseUrl("::", 8080)).toBe("http://[::1]:8080/v1");
    expect(clientApiBaseUrl("::1", 8080)).toBe("http://[::1]:8080/v1");
  });

  it("prefers explicit aliases and falls back to the exact model path", () => {
    expect(
      profileModelIdentifiers(
        profile({ modelAlias: { mode: "custom", value: "coder, local-coder" } }),
      ),
    ).toEqual(["coder", "local-coder"]);
    expect(profileModelIdentifiers(profile())).toEqual(["E:\\models\\coder.gguf"]);
  });

  it("builds copy-ready configurations for supported coding agents", () => {
    const values = {
      apiBaseUrl: "http://127.0.0.1:8080/v1",
      apiKey: "no-key",
      model: "coder",
    };
    expect(buildAgentSnippet("aider", values)).toContain(
      "aider --model 'openai/coder'",
    );
    expect(buildAgentSnippet("openaiJs", values)).toContain(
      'baseURL: "http://127.0.0.1:8080/v1"',
    );
    expect(JSON.parse(buildAgentSnippet("json", values))).toEqual({
      baseURL: values.apiBaseUrl,
      apiKey: "no-key",
      model: "coder",
    });

    const opencode = JSON.parse(buildAgentSnippet("opencode", values));
    expect(opencode.model).toBe("llamapilot/coder");
    expect(opencode.provider.llamapilot).toMatchObject({
      npm: "@ai-sdk/openai-compatible",
      options: {
        baseURL: values.apiBaseUrl,
        apiKey: "no-key",
      },
      models: {
        coder: { name: "coder" },
      },
    });

    const pi = JSON.parse(buildAgentSnippet("pi", values));
    expect(pi.providers.llamapilot).toMatchObject({
      baseUrl: values.apiBaseUrl,
      api: "openai-completions",
      apiKey: "no-key",
      models: [{ id: "coder", name: "coder" }],
    });
  });
});


describe("POSIX Aider snippet", () => {
  it("quotes shell metacharacters and confines environment to Aider", () => {
    const text = buildAgentSnippet("aiderPosix", { apiBaseUrl: "http://localhost:8080/v1", apiKey: "it's $HOME", model: "model; echo unsafe" });
    expect(text).toContain("env OPENAI_API_BASE='");
    expect(text).toContain("OPENAI_API_KEY='it'\"'\"'s $HOME'");
    expect(text).toContain("--model 'openai/model; echo unsafe'");
  });
});
