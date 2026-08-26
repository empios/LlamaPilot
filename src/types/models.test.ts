import { describe, expect, it } from "vitest";

import { modelCatalogSchema } from "@/types/models";

const metadata = {
  version: 3,
  tensorCount: 291,
  metadataCount: 32,
  metadataBytes: 4096,
  generalType: "model",
  architecture: "llama",
  name: "Tiny",
  basename: "tiny",
  sizeLabel: "1B",
  fileType: 15,
  quantizationVersion: 2,
  contextLength: 8192,
  embeddingLength: 2048,
  blockCount: 16,
  tokenizerModel: "llama",
  splitIndex: 0,
  splitCount: 2,
  splitTensorCount: 291,
  projectorType: null,
  hasVisionEncoder: null,
  hasAudioEncoder: null,
};

const catalog = {
  scannedAt: "2026-08-25T20:00:00Z",
  roots: ["E:\\models"],
  models: [
    {
      id: "E:\\models\\tiny.gguf",
      displayName: "Tiny",
      primaryPath: "E:\\models\\tiny-00001-of-00002.gguf",
      directory: "E:\\models",
      shards: [
        {
          path: "E:\\models\\tiny-00001-of-00002.gguf",
          index: 1,
          sizeBytes: 1024,
        },
      ],
      expectedShards: 2,
      complete: false,
      totalSizeBytes: 1024,
      modifiedAt: "2026-08-25T20:00:00Z",
      metadata,
      projectorStatus: "ambiguous",
      projectorId: null,
      projectorCandidates: ["E:\\models\\mmproj-tiny-f16.gguf"],
    },
  ],
  projectors: [],
  issues: [],
  cacheHits: 0,
  cacheMisses: 1,
};

describe("model catalog schema", () => {
  it("accepts incomplete logical models and nullable selections", () => {
    const parsed = modelCatalogSchema.parse(catalog);
    expect(parsed.models[0]?.complete).toBe(false);
    expect(parsed.models[0]?.projectorStatus).toBe("ambiguous");
  });

  it("rejects projector states not understood by the UI", () => {
    const invalid = structuredClone(catalog);
    invalid.models[0]!.projectorStatus = "guessed";
    expect(modelCatalogSchema.safeParse(invalid).success).toBe(false);
  });
});
