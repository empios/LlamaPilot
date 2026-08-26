import {
  AlertTriangleIcon,
  CheckIcon,
  CopyIcon,
  CpuIcon,
  DatabaseIcon,
  NetworkIcon,
  PlusIcon,
  RefreshCwIcon,
  SlidersHorizontalIcon,
  SparklesIcon,
  TrashIcon,
} from "lucide-react";
import { useMemo, useState, type ReactNode } from "react";
import { toast } from "sonner";

import { ErrorPanel } from "@/components/error-panel";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { useRuntimeCapabilities, useInspectRuntimeCapabilities } from "@/hooks/use-build";
import { useDebouncedValue } from "@/hooks/use-debounced-value";
import {
  useCreateProfile,
  useProfilePreview,
  useUpdateProfile,
} from "@/hooks/use-profiles";
import { backendLabel, type RuntimeRecord } from "@/types/build";
import type { LlamaCapabilities, LlamaOption } from "@/types/capabilities";
import type { ModelCatalog, ModelRecord } from "@/types/models";
import {
  createProfileInput,
  profileToInput,
  type CommandPreview,
  type LaunchProfile,
  type ProfileInput,
  type ProfileOptionSetting,
} from "@/types/profiles";
import type { Settings } from "@/types/settings";

import {
  isTensorMode,
  KV_KEYS,
  MEMORY_GPU_KEYS,
  MULTI_GPU_KEYS,
  optionChoices,
  selectedSpeculativeTypes,
  SPECULATIVE_KEYS,
  speculativeControlGroups,
  usesBlockDraftStrategy,
} from "./phase-seven";

const CORE_FLAGS = new Set([
  "-m",
  "--model",
  "--host",
  "--port",
  "-mm",
  "--mmproj",
  "-h",
  "--help",
  "--usage",
  "--version",
  "--list-devices",
]);

export function ProfileEditor({
  profile,
  settings,
  runtimes,
  catalog,
  onClose,
}: {
  profile: LaunchProfile | null;
  settings: Settings;
  runtimes: RuntimeRecord[];
  catalog: ModelCatalog;
  onClose: () => void;
}) {
  const usableRuntime = runtimes.find((runtime) => runtime.capabilities !== null);
  const usableModel = catalog.models.find((model) => model.complete);
  const [draft, setDraft] = useState<ProfileInput>(() =>
    profile
      ? profileToInput(profile)
      : createProfileInput({
          host: settings.server.defaultHost,
          port: settings.server.defaultPort,
          autoSelectPort: settings.server.autoSelectPort,
          runtimeId: usableRuntime?.id,
          modelId: usableModel?.id,
        }),
  );
  const [advancedSearch, setAdvancedSearch] = useState("");
  const selectedRuntime = runtimes.find((runtime) => runtime.id === draft.runtimeId) ?? null;
  const capabilities = useRuntimeCapabilities(
    selectedRuntime?.id ?? null,
    selectedRuntime?.capabilities !== null,
  );
  const inspectRuntime = useInspectRuntimeCapabilities();
  const createProfile = useCreateProfile();
  const updateProfile = useUpdateProfile();
  const debouncedDraft = useDebouncedValue(draft, 300);
  const previewEnabled =
    debouncedDraft.name.trim().length > 0 &&
    debouncedDraft.runtimeId.length > 0 &&
    debouncedDraft.modelId.length > 0 &&
    debouncedDraft.host.trim().length > 0 &&
    debouncedDraft.port > 0;
  const preview = useProfilePreview(profile?.id ?? null, debouncedDraft, previewEnabled);

  const optionGroups = useMemo(() => {
    const all = capabilities.data
      ? Object.values(capabilities.data.capabilities.options).filter(
          (option) => !option.aliases.some((alias) => CORE_FLAGS.has(alias)),
        )
      : [];
    const known = all.filter((option) => option.knownKey !== null);
    const advanced = all.filter((option) => option.knownKey === null);
    const keyCounts = new Map<string, number>();
    for (const option of known) {
      if (option.knownKey) {
        keyCounts.set(option.knownKey, (keyCounts.get(option.knownKey) ?? 0) + 1);
      }
    }
    const memoryGpu = known.filter((option) =>
      MEMORY_GPU_KEYS.has(option.knownKey ?? ""),
    );
    const speculative = known.filter((option) =>
      SPECULATIVE_KEYS.has(option.knownKey ?? ""),
    );
    const general = known.filter((option) =>
      !MEMORY_GPU_KEYS.has(option.knownKey ?? "") &&
      !SPECULATIVE_KEYS.has(option.knownKey ?? ""),
    );
    return { general, memoryGpu, speculative, advanced, keyCounts };
  }, [capabilities.data]);

  const savePending = createProfile.isPending || updateProfile.isPending;
  const save = () => {
    if (profile) {
      updateProfile.mutate(
        { id: profile.id, input: draft },
        { onSuccess: onClose },
      );
    } else {
      createProfile.mutate(draft, { onSuccess: onClose });
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="h-[min(880px,calc(100vh-2rem))] sm:max-w-[min(1240px,calc(100vw-2rem))] grid-rows-[auto_minmax(0,1fr)_auto]">
        <DialogHeader>
          <DialogTitle>{profile ? `Edit ${profile.name}` : "New launch profile"}</DialogTitle>
          <DialogDescription>
            Pin one model and runtime. Only options advertised by that runtime can enter the generated command.
          </DialogDescription>
        </DialogHeader>

        <div className="grid min-h-0 gap-4 lg:grid-cols-[minmax(0,1.35fr)_minmax(360px,0.65fr)]">
          <Tabs defaultValue="identity" className="min-h-0 overflow-hidden">
            <TabsList className="flex w-full">
              <TabsTrigger className="px-2" value="identity">Profile</TabsTrigger>
              <TabsTrigger className="px-2" value="known">General</TabsTrigger>
              <TabsTrigger className="px-2" value="memory-gpu">Memory &amp; GPU</TabsTrigger>
              <TabsTrigger className="px-2" value="speculative">Speculative</TabsTrigger>
              <TabsTrigger className="px-2" value="advanced">Advanced</TabsTrigger>
              <TabsTrigger className="px-2" value="environment">Environment</TabsTrigger>
            </TabsList>

            <div className="mt-2 min-h-0 flex-1 overflow-y-auto rounded-lg border border-border p-4">
              <TabsContent value="identity">
                <FieldGroup>
                  <Field>
                    <FieldLabel htmlFor="profile-name">Name</FieldLabel>
                    <Input
                      id="profile-name"
                      value={draft.name}
                      maxLength={120}
                      onChange={(event) => setDraft({ ...draft, name: event.currentTarget.value })}
                    />
                  </Field>
                  <Field>
                    <FieldLabel htmlFor="profile-description">Description</FieldLabel>
                    <Textarea
                      id="profile-description"
                      rows={2}
                      value={draft.description ?? ""}
                      placeholder="Optional notes about this configuration"
                      onChange={(event) =>
                        setDraft({
                          ...draft,
                          description: event.currentTarget.value || null,
                        })
                      }
                    />
                  </Field>
                  <div className="grid gap-4 md:grid-cols-2">
                    <Field>
                      <FieldLabel>Runtime</FieldLabel>
                      <Select
                        value={draft.runtimeId || undefined}
                        onValueChange={(runtimeId) =>
                          setDraft({ ...draft, runtimeId, options: {} })
                        }
                      >
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Choose a runtime" />
                        </SelectTrigger>
                        <SelectContent>
                          {runtimes.map((runtime) => (
                            <SelectItem key={runtime.id} value={runtime.id}>
                              {runtime.branch} @ {runtime.shortCommit} · {backendLabel(runtime.backend)}
                              {runtime.capabilities ? "" : " · not inspected"}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <FieldDescription>
                        The immutable executable and its capability manifest are pinned.
                      </FieldDescription>
                    </Field>
                    <Field>
                      <FieldLabel>Model</FieldLabel>
                      <Select
                        value={draft.modelId || undefined}
                        onValueChange={(modelId) => setDraft({ ...draft, modelId })}
                      >
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Choose a model" />
                        </SelectTrigger>
                        <SelectContent>
                          {catalog.models.map((model) => (
                            <SelectItem key={model.id} value={model.id} disabled={!model.complete}>
                              {model.displayName}
                              {model.complete ? "" : " · incomplete"}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <FieldDescription>
                        Saves the current first shard and projector paths, not only the display name.
                      </FieldDescription>
                    </Field>
                  </div>

                  {selectedRuntime && !selectedRuntime.capabilities ? (
                    <Alert>
                      <CpuIcon />
                      <AlertTitle>Runtime inspection required</AlertTitle>
                      <AlertDescription className="flex items-center justify-between gap-3">
                        <span>Its supported flags must be known before a profile can be generated.</span>
                        <Button
                          size="sm"
                          disabled={inspectRuntime.isPending}
                          onClick={() => inspectRuntime.mutate(selectedRuntime.id)}
                        >
                          <RefreshCwIcon data-icon="inline-start" />
                          Inspect
                        </Button>
                      </AlertDescription>
                    </Alert>
                  ) : null}

                  <div className="grid gap-4 md:grid-cols-[1fr_160px]">
                    <Field>
                      <FieldLabel htmlFor="profile-host">Host</FieldLabel>
                      <Input
                        id="profile-host"
                        value={draft.host}
                        onChange={(event) => setDraft({ ...draft, host: event.currentTarget.value })}
                      />
                    </Field>
                    <Field>
                      <FieldLabel htmlFor="profile-port">Port</FieldLabel>
                      <Input
                        id="profile-port"
                        type="number"
                        min={1}
                        max={65535}
                        value={draft.port}
                        onChange={(event) =>
                          setDraft({ ...draft, port: Number(event.currentTarget.value) })
                        }
                      />
                    </Field>
                  </div>
                  <Field orientation="horizontal">
                    <Switch
                      id="profile-auto-port"
                      checked={draft.autoSelectPort}
                      onCheckedChange={(autoSelectPort) =>
                        setDraft({ ...draft, autoSelectPort })
                      }
                    />
                    <div>
                      <FieldLabel htmlFor="profile-auto-port">Use another free port if occupied</FieldLabel>
                      <FieldDescription>
                        The process supervisor will apply this when server launching lands.
                      </FieldDescription>
                    </div>
                  </Field>
                </FieldGroup>
              </TabsContent>

              <TabsContent value="known">
                {!selectedRuntime ? (
                  <EmptyOptions text="Choose a runtime first." />
                ) : capabilities.isPending ? (
                  <Skeleton className="h-52 w-full" />
                ) : capabilities.isError ? (
                  <ErrorPanel error={capabilities.error} />
                ) : optionGroups.general.length === 0 ? (
                  <EmptyOptions text="This runtime exposes no recognised profile options." />
                ) : (
                  <ProfileOptionList
                    options={optionGroups.general}
                    keyCounts={optionGroups.keyCounts}
                    draft={draft}
                    capabilities={capabilities.data!.capabilities}
                    models={catalog.models}
                    onChange={(options) => setDraft({ ...draft, options })}
                  />
                )}
              </TabsContent>

              <TabsContent value="memory-gpu">
                {!capabilities.data ? (
                  <EmptyOptions text="Choose and inspect a runtime first." />
                ) : (
                  <MemoryGpuControls
                    options={optionGroups.memoryGpu}
                    keyCounts={optionGroups.keyCounts}
                    draft={draft}
                    capabilities={capabilities.data.capabilities}
                    models={catalog.models}
                    onChange={(options) => setDraft({ ...draft, options })}
                  />
                )}
              </TabsContent>

              <TabsContent value="speculative">
                {!capabilities.data ? (
                  <EmptyOptions text="Choose and inspect a runtime first." />
                ) : (
                  <SpeculativeControls
                    options={optionGroups.speculative}
                    keyCounts={optionGroups.keyCounts}
                    draft={draft}
                    capabilities={capabilities.data.capabilities}
                    models={catalog.models}
                    onChange={(options) => setDraft({ ...draft, options })}
                  />
                )}
              </TabsContent>

              <TabsContent value="advanced" className="space-y-5">
                <Field>
                  <FieldLabel htmlFor="advanced-search">Options discovered from --help</FieldLabel>
                  <Input
                    id="advanced-search"
                    value={advancedSearch}
                    placeholder="Filter by name or flag"
                    onChange={(event) => setAdvancedSearch(event.currentTarget.value)}
                  />
                </Field>
                {capabilities.data ? (
                  <div className="flex flex-col gap-3">
                    {optionGroups.advanced
                      .filter((option) => optionMatches(option, advancedSearch))
                      .map((option) => (
                        <ProfileOptionControl
                          key={option.flag}
                          option={option}
                          optionKey={option.flag}
                          setting={draft.options[option.flag] ?? { mode: "default" }}
                          capabilities={capabilities.data.capabilities}
                          models={catalog.models}
                          onChange={(setting) =>
                            setDraft({
                              ...draft,
                              options: updateOption(draft.options, option.flag, setting),
                            })
                          }
                        />
                      ))}
                  </div>
                ) : null}
                <Field>
                  <FieldLabel htmlFor="additional-arguments">Additional llama.cpp arguments</FieldLabel>
                  <Textarea
                    id="additional-arguments"
                    className="min-h-32 font-mono text-xs"
                    value={draft.additionalArguments.join("\n")}
                    placeholder={"--future-flag\nvalue kept as one argument"}
                    onChange={(event) =>
                      setDraft({
                        ...draft,
                        additionalArguments: event.currentTarget.value
                          .split("\n")
                          .filter((argument) => argument.length > 0),
                      })
                    }
                  />
                  <FieldDescription>
                    One argument per line. Values are appended verbatim as discrete arguments and are never parsed as a shell string.
                  </FieldDescription>
                </Field>
              </TabsContent>

              <TabsContent value="environment">
                <EnvironmentEditor
                  environment={draft.environment}
                  onChange={(environment) => setDraft({ ...draft, environment })}
                />
              </TabsContent>
            </div>
          </Tabs>

          <CommandPreviewPanel preview={preview.data} pending={preview.isFetching} error={preview.error} />
        </div>

        <DialogFooter>
          <span className="mr-auto self-center text-xs text-muted-foreground">
            {profile ? `Saved paths remain pinned until model selection changes.` : "Saved as one human-readable JSON file."}
          </span>
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button disabled={savePending || !preview.data} onClick={save}>
            {savePending ? "Saving…" : profile ? "Save profile" : "Create profile"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ProfileOptionList({
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
            models={models}
            onChange={(setting) =>
              onChange(updateProfileOption(draft.options, option, key, setting))
            }
          />
        );
      })}
    </div>
  );
}

function MemoryGpuControls({
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

function SpeculativeControls({
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
  const typeOptions = orderedKnownOptions(options, ["speculativeType"]);
  const selectedTypes = selectedSpeculativeTypes(draft.options);
  const groups = speculativeControlGroups(selectedTypes);
  const changeSpeculativeOptions = (
    nextOptions: Record<string, ProfileOptionSetting>,
  ) => onChange(pruneInactiveSpeculativeOptions(nextOptions, capabilities));

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
        const groupOptions = orderedKnownOptions(options, group.keys);
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

function ProfileOptionControl({
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
            <Select
              value={setting.value || undefined}
              onValueChange={(value) => onChange({ mode: "custom", value })}
            >
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Choose a complete draft model" />
              </SelectTrigger>
              <SelectContent>
                {models
                  .filter((model) => model.complete && model.primaryPath)
                  .map((model) => (
                    <SelectItem key={model.id} value={model.primaryPath!}>
                      {model.displayName}
                    </SelectItem>
                  ))}
              </SelectContent>
            </Select>
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
          {["device", "draftDevice"].includes(option.knownKey ?? "") && capabilities.devices.length > 0 ? (
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
  const selectedValues = new Set(selected.split(",").map((value) => value.trim()).filter(Boolean));
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

function EnvironmentEditor({
  environment,
  onChange,
}: {
  environment: Record<string, string>;
  onChange: (environment: Record<string, string>) => void;
}) {
  const entries = Object.entries(environment);
  const addVariable = () => {
    let index = entries.length + 1;
    let key = `VARIABLE_${index}`;
    while (Object.hasOwn(environment, key)) {
      key = `VARIABLE_${++index}`;
    }
    onChange({ ...environment, [key]: "" });
  };

  return (
    <FieldGroup>
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-medium">Per-process environment</p>
          <p className="text-xs text-muted-foreground">
            These values are attached only to llama-server and never written to the global environment.
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={addVariable}>
          <PlusIcon data-icon="inline-start" />
          Add variable
        </Button>
      </div>
      {entries.length === 0 ? (
        <div className="rounded-lg border border-dashed p-5 text-center text-sm text-muted-foreground">
          No environment overrides.
        </div>
      ) : (
        entries.map(([key, value], index) => (
          <div key={`${index}-${key}`} className="grid gap-2 md:grid-cols-[minmax(160px,0.7fr)_1fr_auto]">
            <Input
              aria-label={`Environment variable ${index + 1} name`}
              className="font-mono text-xs"
              value={key}
              onChange={(event) => {
                const next = Object.fromEntries(entries.map(([entryKey, entryValue], entryIndex) =>
                  entryIndex === index ? [event.currentTarget.value, entryValue] : [entryKey, entryValue],
                ));
                onChange(next);
              }}
            />
            <Input
              aria-label={`Environment variable ${key} value`}
              className="font-mono text-xs"
              value={value}
              onChange={(event) => onChange({ ...environment, [key]: event.currentTarget.value })}
            />
            <Button
              variant="ghost"
              size="icon"
              onClick={() => onChange(Object.fromEntries(entries.filter((_, entryIndex) => entryIndex !== index)))}
            >
              <TrashIcon />
              <span className="sr-only">Remove {key}</span>
            </Button>
          </div>
        ))
      )}
    </FieldGroup>
  );
}

function CommandPreviewPanel({
  preview,
  pending,
  error,
}: {
  preview: CommandPreview | undefined;
  pending: boolean;
  error: unknown;
}) {
  const [format, setFormat] = useState("command");
  const content = format === "powershell" ? preview?.powershell : preview?.plain;
  return (
    <section className="flex min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-card">
      <header className="flex items-center justify-between gap-3 border-b border-border px-3 py-2.5">
        <div>
          <p className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">Generated command</p>
          <p className="text-xs text-muted-foreground">Display-only; execution uses the argument array.</p>
        </div>
        {pending ? <RefreshCwIcon className="size-4 animate-spin text-muted-foreground" /> : null}
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto p-3">
        {!preview && pending ? <Skeleton className="h-48 w-full" /> : null}
        {error ? <ErrorPanel error={error} /> : null}
        {!preview && !pending && !error ? (
          <div className="flex min-h-48 flex-col items-center justify-center gap-2 rounded-lg border border-dashed p-5 text-center text-sm text-muted-foreground">
            <SlidersHorizontalIcon className="size-5" />
            Choose a runtime and complete model to generate the command.
          </div>
        ) : null}
        {preview ? (
          <div className="space-y-3">
            <div className="flex flex-wrap gap-1.5">
              <Badge variant="secondary">{preview.runtimeLabel}</Badge>
              <Badge variant="outline">{preview.modelName}</Badge>
              <Badge variant="outline">{preview.capabilityVersion}</Badge>
            </div>
            <Tabs value={format} onValueChange={setFormat}>
              <div className="flex items-center justify-between gap-2">
                <TabsList>
                  <TabsTrigger value="command">Command</TabsTrigger>
                  <TabsTrigger value="powershell">PowerShell</TabsTrigger>
                </TabsList>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => content && void copyText(content)}
                >
                  <CopyIcon data-icon="inline-start" />
                  Copy
                </Button>
              </div>
              <TabsContent value="command">
                <pre className="mt-2 max-h-72 overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap break-all">
                  {preview.plain}
                </pre>
              </TabsContent>
              <TabsContent value="powershell">
                <pre className="mt-2 max-h-72 overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap break-all">
                  {preview.powershell}
                </pre>
              </TabsContent>
            </Tabs>
            {Object.keys(preview.environment).length > 0 ? (
              <div>
                <p className="mb-1 text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">Environment</p>
                {Object.entries(preview.environment).map(([key, value]) => (
                  <code key={key} className="block truncate font-mono text-xs">{key}={value}</code>
                ))}
              </div>
            ) : null}
            {preview.warnings.map((warning) => (
              <Alert key={warning}>
                <AlertTriangleIcon />
                <AlertTitle>Launch-time adjustment</AlertTitle>
                <AlertDescription>{warning}</AlertDescription>
              </Alert>
            ))}
          </div>
        ) : null}
      </div>
    </section>
  );
}

function EmptyOptions({ text }: { text: string }) {
  return (
    <div className="flex min-h-40 flex-col items-center justify-center gap-2 rounded-lg border border-dashed text-center text-sm text-muted-foreground">
      <SlidersHorizontalIcon className="size-5" />
      {text}
    </div>
  );
}

function updateOption(
  options: Record<string, ProfileOptionSetting>,
  key: string,
  setting: ProfileOptionSetting,
): Record<string, ProfileOptionSetting> {
  const next = { ...options };
  if (setting.mode === "default") delete next[key];
  else next[key] = setting;
  return next;
}

function settingForOption(
  options: Record<string, ProfileOptionSetting>,
  option: LlamaOption,
  canonicalKey: string,
): ProfileOptionSetting {
  for (const key of [canonicalKey, option.flag, ...option.aliases]) {
    if (Object.hasOwn(options, key)) return options[key]!;
  }
  return { mode: "default" };
}

function updateProfileOption(
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

function pruneInactiveSpeculativeOptions(
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
    const option = Object.values(capabilities.options).find((candidate) =>
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

function profileOptionKey(option: LlamaOption, counts: Map<string, number>): string {
  return option.knownKey && counts.get(option.knownKey) === 1 ? option.knownKey : option.flag;
}

function orderedKnownOptions(
  options: LlamaOption[],
  keys: readonly string[],
): LlamaOption[] {
  return keys.flatMap((key) =>
    options.filter((option) => option.knownKey === key),
  );
}

function optionSupportsAuto(option: LlamaOption): boolean {
  return (option.valueHint ?? "")
    .split(/[^a-zA-Z0-9]+/)
    .some((word) => word.toLowerCase() === "auto");
}

function optionMatches(option: LlamaOption, query: string): boolean {
  const normalized = query.trim().toLowerCase();
  return !normalized || `${option.displayName} ${option.flag} ${option.description}`.toLowerCase().includes(normalized);
}

async function copyText(value: string) {
  try {
    await navigator.clipboard.writeText(value);
    toast.success("Command copied");
  } catch (error) {
    toast.error("Could not copy the command", {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}
