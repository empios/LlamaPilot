import type { LlamaCapabilities, LlamaOption } from "@/types/capabilities";
import type { ProfileOptionSetting } from "@/types/profiles";

interface PresetOverride {
  alias: string;
  value: string;
  draftOnly?: boolean;
}

const SINGLE_USER_131K_OVERRIDES: PresetOverride[] = [
  { alias: "--parallel", value: "1" },
  { alias: "--ctx-size", value: "131072" },
  { alias: "--batch-size", value: "512" },
  { alias: "--ubatch-size", value: "256" },
  { alias: "--gpu-layers", value: "all" },
  { alias: "--flash-attn", value: "on" },
  { alias: "--cache-type-k", value: "q4_0" },
  { alias: "--cache-type-v", value: "q4_0" },
  { alias: "--fit", value: "off" },
  { alias: "--jinja", value: "true" },
  { alias: "--reasoning", value: "on" },
  { alias: "--reasoning-budget", value: "8192" },
  { alias: "--temp", value: "0.6" },
  { alias: "--top-p", value: "0.95" },
  { alias: "--top-k", value: "20" },
  { alias: "--min-p", value: "0" },
  { alias: "--presence-penalty", value: "0" },
  { alias: "--repeat-penalty", value: "1.0" },
  { alias: "--spec-draft-ngl", value: "all", draftOnly: true },
  { alias: "--spec-draft-n-max", value: "2", draftOnly: true },
];

export interface AppliedProfilePreset {
  options: Record<string, ProfileOptionSetting>;
  appliedFlags: string[];
  unavailableFlags: string[];
}

export function applySingleUser131kPreset(
  current: Record<string, ProfileOptionSetting>,
  capabilities: LlamaCapabilities,
): AppliedProfilePreset {
  const options = { ...current };
  const appliedFlags: string[] = [];
  const unavailableFlags: string[] = [];
  const hasDraftStrategy = selectedStrategies(current).some((strategy) =>
    strategy.startsWith("draft-"),
  );
  const knownKeyCounts = countKnownKeys(capabilities);

  for (const override of SINGLE_USER_131K_OVERRIDES) {
    if (override.draftOnly && !hasDraftStrategy) continue;
    const option = findByAlias(capabilities, override.alias);
    if (!option) {
      unavailableFlags.push(override.alias);
      continue;
    }
    setOption(options, option, optionKey(option, knownKeyCounts), override.value);
    appliedFlags.push(option.flag);
  }

  return { options, appliedFlags, unavailableFlags };
}

function selectedStrategies(
  options: Record<string, ProfileOptionSetting>,
): string[] {
  const setting = options.speculativeType ?? options["--spec-type"];
  if (!setting || setting.mode !== "custom") return [];
  return setting.value.split(",").map((value) => value.trim()).filter(Boolean);
}

function findByAlias(
  capabilities: LlamaCapabilities,
  alias: string,
): LlamaOption | null {
  return Object.values(capabilities.options).find((option) =>
    option.aliases.includes(alias),
  ) ?? null;
}

function countKnownKeys(capabilities: LlamaCapabilities): Map<string, number> {
  const counts = new Map<string, number>();
  for (const option of Object.values(capabilities.options)) {
    if (!option.knownKey) continue;
    counts.set(option.knownKey, (counts.get(option.knownKey) ?? 0) + 1);
  }
  return counts;
}

function optionKey(option: LlamaOption, counts: Map<string, number>): string {
  return option.knownKey && counts.get(option.knownKey) === 1
    ? option.knownKey
    : option.flag;
}

function setOption(
  options: Record<string, ProfileOptionSetting>,
  option: LlamaOption,
  key: string,
  value: string,
) {
  for (const identity of [key, option.flag, ...option.aliases]) delete options[identity];
  options[key] = { mode: "custom", value };
}
