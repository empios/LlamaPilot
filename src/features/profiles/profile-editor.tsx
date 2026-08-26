import { CpuIcon, RefreshCwIcon } from "lucide-react";
import { useMemo, useState } from "react";
import { toast } from "sonner";

import { ErrorPanel } from "@/components/error-panel";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
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
import type { LlamaOption } from "@/types/capabilities";
import {
  primaryModels,
  type ModelCatalog,
} from "@/types/models";
import {
  createProfileInput,
  profileToInput,
  type LaunchProfile,
  type ProfileInput,
  type ProfileOptionSetting,
} from "@/types/profiles";
import type { Settings } from "@/types/settings";

import {
  MEMORY_GPU_KEYS,
  SPECULATIVE_KEYS,
} from "./phase-seven";
import {
  ApiModelAliasField,
  CommandPreviewPanel,
  EnvironmentEditor,
} from "./profile-editor-panels";
import {
  EmptyOptions,
  ProfileOptionControl,
  ProfileOptionList,
} from "./profile-option-controls";
import {
  MemoryGpuControls,
  SpeculativeControls,
} from "./profile-specialized-controls";
import { applySingleUser131kPreset } from "./profile-presets";

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
  const mainModels = primaryModels(catalog.models);
  const usableModel = mainModels.find((model) => model.complete);
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
    const modelAlias = known.find((option) => option.knownKey === "modelAlias") ?? null;
    const general = known.filter((option) =>
      !MEMORY_GPU_KEYS.has(option.knownKey ?? "") &&
      !SPECULATIVE_KEYS.has(option.knownKey ?? "") &&
      option.knownKey !== "modelAlias",
    );
    return { general, memoryGpu, speculative, modelAlias, advanced, keyCounts };
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
                          {mainModels.map((model) => (
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

                  {optionGroups.modelAlias ? (
                    <ApiModelAliasField
                      option={optionGroups.modelAlias}
                      keyCounts={optionGroups.keyCounts}
                      options={draft.options}
                      onChange={(options) => setDraft({ ...draft, options })}
                    />
                  ) : null}

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
                  <div className="space-y-4">
                    <div className="rounded-lg border border-primary/35 bg-primary/5 p-4">
                      <div className="flex flex-wrap items-center justify-between gap-3">
                        <div>
                          <p className="text-sm font-semibold">Single-user throughput</p>
                          <p className="mt-1 max-w-2xl text-xs text-muted-foreground">
                            Applies one slot, 131K context, full GPU offload, Q4 KV cache,
                            fixed 512/256 batches, and supported draft/chat tuning. Automatic
                            memory fitting is disabled; network binding is unchanged.
                          </p>
                        </div>
                        <Button
                          type="button"
                          size="sm"
                          onClick={() => {
                            const result = applySingleUser131kPreset(
                              draft.options,
                              capabilities.data!.capabilities,
                            );
                            setDraft({ ...draft, options: result.options });
                            toast.success(
                              `Applied ${result.appliedFlags.length} single-user settings.`,
                            );
                          }}
                        >
                          Apply 131K preset
                        </Button>
                      </div>
                    </div>
                    <ProfileOptionList
                      options={optionGroups.general}
                      keyCounts={optionGroups.keyCounts}
                      draft={draft}
                      capabilities={capabilities.data!.capabilities}
                      models={catalog.models}
                      onChange={(options) => setDraft({ ...draft, options })}
                    />
                  </div>
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

function optionMatches(option: LlamaOption, query: string): boolean {
  const normalized = query.trim().toLowerCase();
  return !normalized || `${option.displayName} ${option.flag} ${option.description}`.toLowerCase().includes(normalized);
}
