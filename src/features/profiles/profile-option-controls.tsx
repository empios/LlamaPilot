import { CheckIcon, SlidersHorizontalIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { LlamaCapabilities, LlamaOption } from "@/types/capabilities";
import { drafterModelsFor, type ModelRecord } from "@/types/models";
import type { ProfileInput, ProfileOptionSetting } from "@/types/profiles";

import { optionChoices } from "./phase-seven";

export function ProfileOptionList({
  options,
  keyCounts,
  draft,
  capabilities,
  models,
  onChange,
}: {
  options: LlamaOption[];
  keyCounts: Map<string, number>;
  draft: ProfileInput;
  capabilities: LlamaCapabilities;
  models: ModelRecord[];
  onChange: (options: Record<string, ProfileOptionSetting>) => void;
}) {
  return (
    <div className="flex flex-col gap-3">
      {options.map((option) => {
        const key = profileOptionKey(option, keyCounts);
        const setting = settingForOption(draft.options, option, key);
        return (
          <ProfileOptionControl
            key={option.flag}
            option={option}
            optionKey={key}
            setting={setting}
            capabilities={capabilities}
            models={
              option.knownKey === "draftModel"
                ? drafterModelsFor(models, draft.modelId)
                : models
            }
            onChange={(nextSetting) =>
              onChange(updateProfileOption(draft.options, option, key, nextSetting))
            }
          />
        );
      })}
    </div>
  );
}

export function ProfileOptionControl({
  option,
  optionKey,
  setting,
  capabilities,
  models,
  onChange,
}: {
  option: LlamaOption;
  optionKey: string;
  setting: ProfileOptionSetting;
  capabilities: LlamaCapabilities;
  models: ModelRecord[];
  onChange: (setting: ProfileOptionSetting) => void;
}) {
  const isSwitch = option.valueHint === null;
  const hasNegative = option.aliases.some((alias) => alias.startsWith("--no-"));
  const hasPositive = option.aliases.some((alias) => !alias.startsWith("--no-"));
  const hasPairedSwitches = hasNegative && hasPositive;
  const mode = isSwitch
    ? setting.mode === "custom"
      ? setting.value === "false"
        ? "disabled"
        : "enabled"
      : "default"
    : setting.mode;
  const choices = optionChoices(option).filter(
    (value) => value.toLowerCase() !== "auto",
  );
  const isMultiValue = ["speculativeType", "device", "draftDevice"].includes(
    option.knownKey ?? "",
  );
  const isNumeric =
    option.knownKey?.startsWith("ngram") === true ||
    option.knownKey?.startsWith("specDraft") === true ||
    ["mainGpu"].includes(option.knownKey ?? "");

  return (
    <div className="rounded-lg border border-border p-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium">{option.displayName}</span>
            <code className="font-mono text-xs text-primary">{option.flag}</code>
            {option.valueHint ? <Badge variant="outline">{option.valueHint}</Badge> : null}
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            {option.summary ?? option.description ?? "No description reported."}
          </p>
        </div>
        <Select
          value={mode}
          onValueChange={(nextMode) => {
            if (nextMode === "default") onChange({ mode: "default" });
            else if (nextMode === "auto") onChange({ mode: "auto" });
            else if (nextMode === "enabled") onChange({ mode: "custom", value: "true" });
            else if (nextMode === "disabled") onChange({ mode: "custom", value: "false" });
            else onChange({ mode: "custom", value: "" });
          }}
        >
          <SelectTrigger size="sm" className="w-28">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="default">Default</SelectItem>
            {isSwitch ? (
              <>
                <SelectItem value="enabled">
                  {hasPairedSwitches ? "Enabled" : "Pass flag"}
                </SelectItem>
                {hasPairedSwitches ? <SelectItem value="disabled">Disabled</SelectItem> : null}
              </>
            ) : (
              <>
                {optionSupportsAuto(option) ? <SelectItem value="auto">Auto</SelectItem> : null}
                <SelectItem value="custom">Custom</SelectItem>
              </>
            )}
          </SelectContent>
        </Select>
      </div>

      {!isSwitch && setting.mode === "custom" ? (
        <div className="mt-3">
          {option.knownKey === "draftModel" ? (
            <DraftModelSelect
              models={models}
              value={setting.value}
              onChange={(value) => onChange({ mode: "custom", value })}
            />
          ) : choices.length > 0 && !isMultiValue ? (
            <Select
              value={setting.value || undefined}
              onValueChange={(value) => onChange({ mode: "custom", value })}
            >
              <SelectTrigger className="w-full font-mono text-xs">
                <SelectValue placeholder={`Choose ${option.displayName.toLowerCase()}`} />
              </SelectTrigger>
              <SelectContent>
                {choices.map((value) => (
                  <SelectItem key={value} value={value} className="font-mono text-xs">
                    {value}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : (
            <Input
              className="font-mono text-xs"
              type={isNumeric ? "number" : "text"}
              step={option.knownKey?.includes("Probability") ? "0.01" : undefined}
              value={setting.value}
              placeholder={option.valueHint ?? "value"}
              onChange={(event) =>
                onChange({ mode: "custom", value: event.currentTarget.value })
              }
            />
          )}
          {option.knownKey === "speculativeType" && capabilities.speculativeTypes.length > 0 ? (
            <SuggestionButtons
              values={capabilities.speculativeTypes}
              selected={setting.value}
              exclusiveNone
              onChange={(value) => onChange({ mode: "custom", value })}
            />
          ) : null}
          {["device", "draftDevice"].includes(option.knownKey ?? "") &&
          capabilities.devices.length > 0 ? (
            <SuggestionButtons
              values={capabilities.devices.map((device) => device.id)}
              selected={setting.value}
              onChange={(value) => onChange({ mode: "custom", value })}
            />
          ) : null}
        </div>
      ) : null}
      <span className="sr-only">Profile option key {optionKey}</span>
    </div>
  );
}

export function DraftModelSelect({
  models,
  value,
  onChange,
}: {
  models: ModelRecord[];
  value: string;
  onChange: (value: string) => void;
}) {
  const savedModelIsVisible = models.some((model) => model.primaryPath === value);

  if (models.length === 0 && !value) {
    return (
      <div className="rounded-md border border-dashed px-3 py-2 text-xs text-muted-foreground">
        No compatible drafter was detected for the selected primary model. Add its GGUF file to a
        configured model folder and scan again.
      </div>
    );
  }

  return (
    <Select value={value || undefined} onValueChange={onChange}>
      <SelectTrigger className="w-full">
        <SelectValue placeholder="Choose a compatible drafter" />
      </SelectTrigger>
      <SelectContent>
        {value && !savedModelIsVisible ? (
          <SelectItem value={value}>Saved draft file</SelectItem>
        ) : null}
        {models.map((model) => (
          <SelectItem key={model.id} value={model.primaryPath!}>
            {model.displayName}
            {model.metadata.architecture ? ` · ${model.metadata.architecture}` : ""}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

function SuggestionButtons({
  values,
  selected,
  exclusiveNone = false,
  onChange,
}: {
  values: string[];
  selected: string;
  exclusiveNone?: boolean;
  onChange: (value: string) => void;
}) {
  const selectedValues = new Set(
    selected
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean),
  );
  return (
    <div className="mt-2 flex flex-wrap gap-1.5">
      {values.map((value) => {
        const active = selectedValues.has(value);
        return (
          <Button
            key={value}
            type="button"
            variant={active ? "secondary" : "outline"}
            size="sm"
            className="h-6 px-2 font-mono text-[11px]"
            onClick={() => {
              if (active) {
                selectedValues.delete(value);
              } else if (exclusiveNone && value === "none") {
                selectedValues.clear();
                selectedValues.add(value);
              } else {
                if (exclusiveNone) selectedValues.delete("none");
                selectedValues.add(value);
              }
              onChange([...selectedValues].join(","));
            }}
          >
            {active ? <CheckIcon data-icon="inline-start" /> : null}
            {value}
          </Button>
        );
      })}
    </div>
  );
}

export function EmptyOptions({ text }: { text: string }) {
  return (
    <div className="flex min-h-40 flex-col items-center justify-center gap-2 rounded-lg border border-dashed text-center text-sm text-muted-foreground">
      <SlidersHorizontalIcon className="size-5" />
      {text}
    </div>
  );
}

export function settingForOption(
  options: Record<string, ProfileOptionSetting>,
  option: LlamaOption,
  canonicalKey: string,
): ProfileOptionSetting {
  for (const key of [canonicalKey, option.flag, ...option.aliases]) {
    if (Object.hasOwn(options, key)) return options[key]!;
  }
  return { mode: "default" };
}

export function updateProfileOption(
  options: Record<string, ProfileOptionSetting>,
  option: LlamaOption,
  canonicalKey: string,
  setting: ProfileOptionSetting,
): Record<string, ProfileOptionSetting> {
  const next = { ...options };
  for (const key of [canonicalKey, option.flag, ...option.aliases]) delete next[key];
  if (setting.mode !== "default") next[canonicalKey] = setting;
  return next;
}

export function profileOptionKey(option: LlamaOption, counts: Map<string, number>): string {
  return option.knownKey && counts.get(option.knownKey) === 1 ? option.knownKey : option.flag;
}

export function orderedKnownOptions(
  options: LlamaOption[],
  keys: readonly string[],
): LlamaOption[] {
  return keys.flatMap((key) => options.filter((option) => option.knownKey === key));
}

function optionSupportsAuto(option: LlamaOption): boolean {
  return (option.valueHint ?? "")
    .split(/[^a-zA-Z0-9]+/)
    .some((word) => word.toLowerCase() === "auto");
}
