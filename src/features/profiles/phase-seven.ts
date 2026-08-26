import type { LlamaOption } from "@/types/capabilities";
import type { ProfileOptionSetting } from "@/types/profiles";

export const MEMORY_GPU_KEYS = new Set([
  "kvCacheTypeK",
  "kvCacheTypeV",
  "kvCacheOffload",
  "kvUnified",
  "swaFull",
  "gpuLayers",
  "device",
  "splitMode",
  "tensorSplit",
  "mainGpu",
  "flashAttention",
  "fit",
]);

export const SPECULATIVE_KEYS = new Set([
  "speculativeType",
  "draftModel",
  "draftDevice",
  "draftGpuLayers",
  "draftKvCacheTypeK",
  "draftKvCacheTypeV",
  "specDraftMaxTokens",
  "specDraftMinTokens",
  "specDraftSplitProbability",
  "specDraftMinProbability",
  "specDraftBackendSampling",
  "ngramModMinTokens",
  "ngramModMaxTokens",
  "ngramModMatchTokens",
  "ngramSimpleSizeN",
  "ngramSimpleSizeM",
  "ngramSimpleMinHits",
  "ngramMapKSizeN",
  "ngramMapKSizeM",
  "ngramMapKMinHits",
  "ngramMapK4vSizeN",
  "ngramMapK4vSizeM",
  "ngramMapK4vMinHits",
]);

export const KV_KEYS = [
  "kvCacheTypeK",
  "kvCacheTypeV",
  "kvCacheOffload",
  "kvUnified",
  "swaFull",
] as const;

export const MULTI_GPU_KEYS = [
  "gpuLayers",
  "device",
  "splitMode",
  "tensorSplit",
  "mainGpu",
  "flashAttention",
  "fit",
] as const;

const EXTERNAL_DRAFT_TYPES = new Set([
  "draft-simple",
  "draft-eagle3",
  "draft-dflash",
  "draft-dspark",
]);

export interface SpeculativeControlGroup {
  id: string;
  title: string;
  description: string;
  keys: string[];
}

export function selectedSpeculativeTypes(
  options: Record<string, ProfileOptionSetting>,
): string[] {
  const setting = options.speculativeType ?? options["--spec-type"];
  if (!setting || setting.mode !== "custom") return [];
  return [...new Set(
    setting.value
      .split(",")
      .map((value) => value.trim())
      .filter((value) => value && value !== "none"),
  )];
}

export function speculativeControlGroups(
  selectedTypes: string[],
): SpeculativeControlGroup[] {
  const selected = new Set(selectedTypes);
  const groups: SpeculativeControlGroup[] = [];

  if ([...EXTERNAL_DRAFT_TYPES].some((strategy) => selected.has(strategy))) {
    groups.push({
      id: "draft-model",
      title: "Draft model",
      description: "Placement and KV settings for the secondary GGUF model.",
      keys: [
        "draftModel",
        "draftDevice",
        "draftGpuLayers",
        "draftKvCacheTypeK",
        "draftKvCacheTypeV",
      ],
    });
  }

  if (selectedTypes.some((strategy) => strategy.startsWith("draft-"))) {
    groups.push({
      id: "draft-tuning",
      title: "Draft tuning",
      description: "Candidate length, probability thresholds, and sampling placement.",
      keys: [
        "specDraftMaxTokens",
        "specDraftMinTokens",
        "specDraftSplitProbability",
        "specDraftMinProbability",
        "specDraftBackendSampling",
      ],
    });
  }

  if (selected.has("ngram-cache")) {
    groups.push({
      id: "ngram-cache",
      title: "N-gram cache",
      description: "History-cache speculation uses the shared maximum draft length.",
      keys: ["specDraftMaxTokens"],
    });
  }
  if (selected.has("ngram-simple")) {
    groups.push({
      id: "ngram-simple",
      title: "ngram-simple",
      description: "Independent lookup length, draft length, and minimum-hit threshold.",
      keys: [
        "specDraftMaxTokens",
        "ngramSimpleSizeN",
        "ngramSimpleSizeM",
        "ngramSimpleMinHits",
      ],
    });
  }
  if (selected.has("ngram-map-k")) {
    groups.push({
      id: "ngram-map-k",
      title: "ngram-map-k",
      description: "Keyed n-gram lookup with its own sizing and hit threshold.",
      keys: [
        "specDraftMaxTokens",
        "ngramMapKSizeN",
        "ngramMapKSizeM",
        "ngramMapKMinHits",
      ],
    });
  }
  if (selected.has("ngram-map-k4v")) {
    groups.push({
      id: "ngram-map-k4v",
      title: "ngram-map-k4v",
      description: "Experimental four-value keyed lookup with isolated sizing controls.",
      keys: [
        "specDraftMaxTokens",
        "ngramMapK4vSizeN",
        "ngramMapK4vSizeM",
        "ngramMapK4vMinHits",
      ],
    });
  }
  if (selected.has("ngram-mod")) {
    groups.push({
      id: "ngram-mod",
      title: "ngram-mod",
      description: "This strategy has its own match, minimum, and maximum token counts.",
      keys: [
        "ngramModMatchTokens",
        "ngramModMinTokens",
        "ngramModMaxTokens",
      ],
    });
  }

  const claimed = new Set<string>();
  return groups
    .map((group) => ({
      ...group,
      keys: group.keys.filter((key) => {
        if (claimed.has(key)) return false;
        claimed.add(key);
        return true;
      }),
    }))
    .filter((group) => group.keys.length > 0);
}

export function optionChoices(option: LlamaOption): string[] {
  const allowedLine = option.description
    .split("\n")
    .find((line) => line.trim().toLowerCase().startsWith("allowed values:"));
  if (allowedLine) {
    return allowedLine
      .slice(allowedLine.indexOf(":") + 1)
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean);
  }

  const rawHint = (option.valueHint ?? "").trim();
  if (!/^[<[{].*[>\]}]$/.test(rawHint)) return [];
  const hint = rawHint.replace(/^[<[{]|[>\]}]$/g, "");
  if (!hint.includes("|") && !hint.includes(",")) return [];
  const choices = hint
    .split(/[|,]/)
    .map((value) => value.trim())
    .filter((value) => /^[a-zA-Z0-9_-]+$/.test(value));
  return choices.length > 1 ? choices : [];
}

export function isTensorMode(
  options: Record<string, ProfileOptionSetting>,
): boolean {
  const setting = options.splitMode ?? options["--split-mode"];
  return setting?.mode === "custom" && setting.value.trim().toLowerCase() === "tensor";
}

export function usesBlockDraftStrategy(selectedTypes: string[]): boolean {
  return selectedTypes.some((strategy) =>
    strategy === "draft-dflash" || strategy === "draft-dspark");
}
