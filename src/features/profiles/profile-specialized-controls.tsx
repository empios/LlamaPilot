import {
  AlertTriangleIcon,
  DatabaseIcon,
  NetworkIcon,
  SparklesIcon,
} from "lucide-react";
import type { ReactNode } from "react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import type { LlamaCapabilities, LlamaOption } from "@/types/capabilities";
import {
  drafterModelsFor,
  draftStrategyFor,
  type ModelRecord,
} from "@/types/models";
import type { ProfileInput, ProfileOptionSetting } from "@/types/profiles";

import {
  isExternalDraftStrategy,
  isTensorMode,
  KV_KEYS,
  MULTI_GPU_KEYS,
  selectedSpeculativeTypes,
  SPECULATIVE_KEYS,
  speculativeControlGroups,
  usesBlockDraftStrategy,
} from "./phase-seven";
import {
  DraftModelSelect,
  EmptyOptions,
  orderedKnownOptions,
  ProfileOptionList,
  profileOptionKey,
  settingForOption,
  updateProfileOption,
} from "./profile-option-controls";

interface SpecializedControlsProps {
  options: LlamaOption[];
  keyCounts: Map<string, number>;
  draft: ProfileInput;
  capabilities: LlamaCapabilities;
  models: ModelRecord[];
  onChange: (options: Record<string, ProfileOptionSetting>) => void;
}

export function MemoryGpuControls({
  options,
  keyCounts,
  draft,
  capabilities,
  models,
  onChange,
}: SpecializedControlsProps) {
  const kvOptions = orderedKnownOptions(options, KV_KEYS);
  const gpuOptions = orderedKnownOptions(options, MULTI_GPU_KEYS);

  if (options.length === 0) {
    return <EmptyOptions text="This runtime advertises no specialised memory or GPU controls." />;
  }

  return (
    <div className="space-y-6">
      <OptionSectionHeading
        icon={<DatabaseIcon className="size-4" />}
        title="KV cache"
        description="Storage type, accelerator offload, slot sharing, and sliding-window cache policy."
      />
      {kvOptions.length > 0 ? (
        <ProfileOptionList
          options={kvOptions}
          keyCounts={keyCounts}
          draft={draft}
          capabilities={capabilities}
          models={models}
          onChange={onChange}
        />
      ) : (
        <p className="text-xs text-muted-foreground">No KV controls are advertised by this runtime.</p>
      )}

      <OptionSectionHeading
        icon={<NetworkIcon className="size-4" />}
        title="Multi-GPU placement"
        description="Device order, layer placement, split proportions, and memory fitting."
      />
      {isTensorMode(draft.options) ? (
        <Alert>
          <AlertTriangleIcon />
          <AlertTitle>Experimental tensor parallelism</AlertTitle>
          <AlertDescription>
            Requires Flash Attention, f32/f16/bf16 KV cache, and a supported model architecture. Automatic fit is unavailable.
          </AlertDescription>
        </Alert>
      ) : null}
      {capabilities.devices.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {capabilities.devices.map((device) => (
            <Badge key={device.id} variant="outline">
              {device.id} · {device.name}
              {device.memoryFreeMib !== null ? ` · ${device.memoryFreeMib} MiB free` : ""}
            </Badge>
          ))}
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">
          This inspection reported no accelerator devices; controls remain limited to flags the binary advertises.
        </p>
      )}
      {gpuOptions.length > 0 ? (
        <ProfileOptionList
          options={gpuOptions}
          keyCounts={keyCounts}
          draft={draft}
          capabilities={capabilities}
          models={models}
          onChange={onChange}
        />
      ) : null}
    </div>
  );
}

export function SpeculativeControls({
  options,
  keyCounts,
  draft,
  capabilities,
  models,
  onChange,
}: SpecializedControlsProps) {
  const typeOptions = orderedKnownOptions(options, ["speculativeType"]);
  const draftModelOption = orderedKnownOptions(options, ["draftModel"])[0] ?? null;
  const selectedTypes = selectedSpeculativeTypes(draft.options);
  const groups = speculativeControlGroups(selectedTypes);
  const directDrafterSupported =
    capabilities.speculativeTypes.some(isExternalDraftStrategy) &&
    typeOptions.length > 0 &&
    draftModelOption !== null;
  const compatibleDrafters = drafterModelsFor(models, draft.modelId).filter(
    (model) => draftStrategyFor(model, capabilities.speculativeTypes) !== null,
  );
  const draftModelSetting = draftModelOption
    ? settingForOption(
        draft.options,
        draftModelOption,
        profileOptionKey(draftModelOption, keyCounts),
      )
    : { mode: "default" as const };
  const changeSpeculativeOptions = (
    nextOptions: Record<string, ProfileOptionSetting>,
  ) => onChange(pruneInactiveSpeculativeOptions(nextOptions, capabilities));
  const configureDrafter = (path: string) => {
    const strategyOption = typeOptions[0];
    if (!strategyOption || !draftModelOption) return;
    const model = compatibleDrafters.find((candidate) => candidate.primaryPath === path);
    if (!model) return;
    const strategy = draftStrategyFor(model, capabilities.speculativeTypes);
    if (!strategy) return;
    let nextOptions = updateProfileOption(
      draft.options,
      strategyOption,
      profileOptionKey(strategyOption, keyCounts),
      { mode: "custom", value: strategy },
    );
    nextOptions = updateProfileOption(
      nextOptions,
      draftModelOption,
      profileOptionKey(draftModelOption, keyCounts),
      { mode: "custom", value: path },
    );
    changeSpeculativeOptions(nextOptions);
  };

  if (typeOptions.length === 0) {
    return <EmptyOptions text="This runtime does not advertise --spec-type." />;
  }

  return (
    <div className="space-y-6">
      <OptionSectionHeading
        icon={<SparklesIcon className="size-4" />}
        title="Strategies"
        description="Only strategy names and flags reported by the selected runtime are available."
      />
      {directDrafterSupported ? (
        <div className="rounded-lg border border-primary/35 bg-primary/5 p-4">
          <div className="mb-3 flex flex-wrap items-start justify-between gap-2">
            <div>
              <p className="text-sm font-semibold">Drafter model</p>
              <p className="mt-1 text-xs text-muted-foreground">
                Choose the assistant GGUF paired with the primary model. MTP, DFlash, DSpark,
                and EAGLE3 files automatically select their matching speculative strategy.
              </p>
            </div>
            {selectedTypes.some(isExternalDraftStrategy) &&
            draftModelSetting.mode === "custom" ? (
              <Badge>Configured</Badge>
            ) : (
              <Badge variant="outline">Optional</Badge>
            )}
          </div>
          <DraftModelSelect
            models={compatibleDrafters}
            value={draftModelSetting.mode === "custom" ? draftModelSetting.value : ""}
            onChange={configureDrafter}
          />
        </div>
      ) : null}
      <ProfileOptionList
        options={typeOptions}
        keyCounts={keyCounts}
        draft={draft}
        capabilities={capabilities}
        models={models}
        onChange={changeSpeculativeOptions}
      />

      {selectedTypes.length === 0 ? (
        <div className="rounded-lg border border-dashed p-5 text-center text-sm text-muted-foreground">
          Choose Custom above and select at least one strategy to reveal its own controls.
        </div>
      ) : null}

      {usesBlockDraftStrategy(selectedTypes) ? (
        <Alert>
          <AlertTriangleIcon />
          <AlertTitle>Draft block-size limit</AlertTitle>
          <AlertDescription>
            DFlash and DSpark clamp maximum draft tokens to the block size stored in the draft model.
          </AlertDescription>
        </Alert>
      ) : null}

      {groups.map((group) => {
        const groupKeys =
          directDrafterSupported && selectedTypes.some(isExternalDraftStrategy)
            ? group.keys.filter((key) => key !== "draftModel")
            : group.keys;
        const groupOptions = orderedKnownOptions(options, groupKeys);
        return (
          <section key={group.id} className="space-y-3">
            <div>
              <p className="text-sm font-semibold">{group.title}</p>
              <p className="text-xs text-muted-foreground">{group.description}</p>
            </div>
            {groupOptions.length > 0 ? (
              <ProfileOptionList
                options={groupOptions}
                keyCounts={keyCounts}
                draft={draft}
                capabilities={capabilities}
                models={models}
                onChange={changeSpeculativeOptions}
              />
            ) : (
              <p className="rounded-lg border border-dashed p-3 text-xs text-muted-foreground">
                This runtime advertises the strategy but none of its optional tuning flags.
              </p>
            )}
          </section>
        );
      })}
    </div>
  );
}

function OptionSectionHeading({
  icon,
  title,
  description,
}: {
  icon: ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="flex items-start gap-2">
      <span className="mt-0.5 text-primary">{icon}</span>
      <div>
        <p className="text-sm font-semibold">{title}</p>
        <p className="text-xs text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}

export function pruneInactiveSpeculativeOptions(
  options: Record<string, ProfileOptionSetting>,
  capabilities: LlamaCapabilities,
): Record<string, ProfileOptionSetting> {
  const allowed = new Set([
    "speculativeType",
    ...speculativeControlGroups(selectedSpeculativeTypes(options)).flatMap(
      (group) => group.keys,
    ),
  ]);
  const next = { ...options };
  for (const storedKey of Object.keys(next)) {
    const option = Object.values(capabilities.options).find(
      (candidate) =>
        candidate.knownKey === storedKey ||
        candidate.flag === storedKey ||
        candidate.aliases.includes(storedKey),
    );
    const knownKey = option?.knownKey;
    if (knownKey && SPECULATIVE_KEYS.has(knownKey) && !allowed.has(knownKey)) {
      delete next[storedKey];
    }
  }
  return next;
}
