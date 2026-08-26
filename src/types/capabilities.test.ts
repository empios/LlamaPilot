import { describe, expect, it } from "vitest";

import { runtimeInspectionSchema } from "@/types/capabilities";

describe("runtimeInspectionSchema", () => {
  it("accepts the typed manifest and preserves unknown runtime flags", () => {
    const parsed = runtimeInspectionSchema.parse({
      capabilities: {
        schemaVersion: 1,
        version: "b7900-790b5713",
        commit: "790b5713",
        options: {
          "--future-flag": {
            flag: "--future-flag",
            aliases: ["--future-flag"],
            valueHint: "VALUE",
            description: "A new upstream option.",
            section: "example-specific params",
            category: "advanced",
            knownKey: null,
            displayName: "Future flag",
            summary: null,
          },
        },
        speculativeTypes: ["none", "draft-simple"],
        devices: [
          {
            id: "CUDA0",
            name: "NVIDIA GeForce RTX 4090",
            backend: "CUDA",
            memoryTotalMib: 24564,
            memoryFreeMib: 22104,
            raw: "CUDA0: NVIDIA GeForce RTX 4090 (24564 MiB, 22104 MiB free)",
          },
        ],
      },
      raw: {
        version: { stdout: "", stderr: "version: b7900-790b5713\n" },
        help: { stdout: "--future-flag VALUE\n", stderr: "" },
        devices: { stdout: "Available devices:\n", stderr: "" },
      },
    });

    expect(parsed.capabilities.options["--future-flag"]?.knownKey).toBeNull();
    expect(parsed.raw.version.stderr).toContain("b7900");
  });

  it("rejects an option category the frontend cannot render", () => {
    const result = runtimeInspectionSchema.safeParse({
      capabilities: {
        schemaVersion: 1,
        version: "b1-deadbee",
        commit: null,
        options: {
          "--x": {
            flag: "--x",
            aliases: ["--x"],
            valueHint: null,
            description: "x",
            section: "common params",
            category: "future-category",
            knownKey: null,
            displayName: "X",
            summary: null,
          },
        },
        speculativeTypes: [],
        devices: [],
      },
      raw: {
        version: { stdout: "", stderr: "" },
        help: { stdout: "", stderr: "" },
        devices: { stdout: "", stderr: "" },
      },
    });

    expect(result.success).toBe(false);
  });
});
