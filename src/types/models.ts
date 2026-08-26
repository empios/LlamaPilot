import { z } from "zod";

export const ggufMetadataSchema = z.object({
  version: z.number().int(),
  tensorCount: z.number().int(),
  metadataCount: z.number().int(),
  metadataBytes: z.number().int(),
  generalType: z.string().nullable(),
  architecture: z.string().nullable(),
  name: z.string().nullable(),
  basename: z.string().nullable(),
  sizeLabel: z.string().nullable(),
  fileType: z.number().int().nullable(),
  quantizationVersion: z.number().int().nullable(),
  contextLength: z.number().int().nullable(),
  embeddingLength: z.number().int().nullable(),
  blockCount: z.number().int().nullable(),
  tokenizerModel: z.string().nullable(),
  splitIndex: z.number().int().nullable(),
  splitCount: z.number().int().nullable(),
  splitTensorCount: z.number().int().nullable(),
  projectorType: z.string().nullable(),
  hasVisionEncoder: z.boolean().nullable(),
  hasAudioEncoder: z.boolean().nullable(),
});

export const modelShardSchema = z.object({
  path: z.string(),
  index: z.number().int(),
  sizeBytes: z.number().int(),
});

export const projectorStatusSchema = z.enum([
  "none",
  "auto",
  "ambiguous",
  "overridden",
  "disabled",
  "missingOverride",
]);

export const modelRoleSchema = z.enum(["main", "drafter"]);

export const modelRecordSchema = z.object({
  id: z.string(),
  displayName: z.string(),
  role: modelRoleSchema,
  primaryPath: z.string().nullable(),
  directory: z.string(),
  shards: z.array(modelShardSchema),
  expectedShards: z.number().int(),
  complete: z.boolean(),
  totalSizeBytes: z.number().int(),
  modifiedAt: z.string(),
  metadata: ggufMetadataSchema,
  projectorStatus: projectorStatusSchema,
  projectorId: z.string().nullable(),
  projectorCandidates: z.array(z.string()),
});

export const projectorRecordSchema = z.object({
  id: z.string(),
  displayName: z.string(),
  path: z.string(),
  directory: z.string(),
  sizeBytes: z.number().int(),
  modifiedAt: z.string(),
  metadata: ggufMetadataSchema,
});

export const modelCatalogSchema = z.object({
  scannedAt: z.string(),
  roots: z.array(z.string()),
  models: z.array(modelRecordSchema),
  projectors: z.array(projectorRecordSchema),
  issues: z.array(
    z.object({
      path: z.string(),
      message: z.string(),
    }),
  ),
  cacheHits: z.number().int(),
  cacheMisses: z.number().int(),
});

export type GgufMetadata = z.infer<typeof ggufMetadataSchema>;
export type ModelRole = z.infer<typeof modelRoleSchema>;
export type ModelRecord = z.infer<typeof modelRecordSchema>;
export type ProjectorRecord = z.infer<typeof projectorRecordSchema>;
export type ModelCatalog = z.infer<typeof modelCatalogSchema>;

export type ProjectorSelection =
  | { mode: "auto" }
  | { mode: "disabled" }
  | { mode: "custom"; path: string };

export function primaryModels(models: ModelRecord[]): ModelRecord[] {
  return models.filter((model) => model.role === "main");
}

export function drafterModelsFor(
  models: ModelRecord[],
  primaryModelId: string,
): ModelRecord[] {
  const drafters = models.filter(
    (model) => model.role === "drafter" && model.complete && model.primaryPath,
  );
  const primary = models.find(
    (model) => model.id === primaryModelId && model.role === "main",
  );
  const primaryArchitecture = baseArchitecture(primary?.metadata.architecture);
  if (!primaryArchitecture) return drafters;

  const compatible = drafters.filter(
    (model) => baseArchitecture(model.metadata.architecture) === primaryArchitecture,
  );
  return compatible.length > 0 ? compatible : drafters;
}

const DRAFT_STRATEGY_MARKERS = [
  { strategy: "draft-dflash", markers: ["dflash"] },
  { strategy: "draft-dspark", markers: ["dspark"] },
  { strategy: "draft-eagle3", markers: ["eagle3", "eagle-3"] },
  { strategy: "draft-mtp", markers: ["mtp"] },
] as const;

export function draftStrategyFor(
  model: ModelRecord,
  supportedTypes: string[],
): string | null {
  const supported = new Set(supportedTypes);
  const identity = [
    model.displayName,
    model.primaryPath,
    model.metadata.name,
    model.metadata.basename,
    model.metadata.architecture,
  ]
    .filter((value): value is string => Boolean(value))
    .join(" ")
    .toLowerCase();

  const marked = DRAFT_STRATEGY_MARKERS.find(({ markers }) =>
    markers.some((marker) => identity.includes(marker)),
  );
  if (marked) return supported.has(marked.strategy) ? marked.strategy : null;

  if (
    /[-_]assistant\b/.test(model.metadata.architecture?.toLowerCase() ?? "")
  ) {
    return supported.has("draft-mtp") ? "draft-mtp" : null;
  }

  const externalTypes = supportedTypes.filter((type) => type.startsWith("draft-"));
  if (externalTypes.length === 1) return externalTypes[0] ?? null;
  if (supported.has("draft-simple")) return "draft-simple";
  return externalTypes[0] ?? null;
}

function baseArchitecture(architecture: string | null | undefined): string | null {
  if (!architecture) return null;
  return architecture.toLowerCase().replace(/[-_]assistant$/, "");
}
