import { CpuIcon, HammerIcon, MicrochipIcon, SquareIcon } from "lucide-react";
import { useState } from "react";

import { OutputConsole } from "@/components/output-console";
import { Section } from "@/components/section";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
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
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import {
  useBuildRuntime,
  useBuildStatus,
  useCancelBuild,
  useToolchain,
} from "@/hooks/use-build";
import { useSettings } from "@/hooks/use-settings";
import { useSources } from "@/hooks/use-sources";
import { createProgressChannel } from "@/lib/ipc";
import { useBuildLogStore } from "@/stores/build-store";
import {
  backendLabel,
  buildBackendSchema,
  buildConfigurationSchema,
  configurationLabel,
  defaultBuildProfile,
  type BuildProfile,
} from "@/types/build";

const CMAKE_DEFAULT_GENERATOR = "__cmake_default__";

export function BuildForm() {
  const sources = useSources();
  const toolchain = useToolchain();
  const buildRuntime = useBuildRuntime();
  const buildStatus = useBuildStatus();
  const cancelBuild = useCancelBuild();
  const settings = useSettings();

  const lines = useBuildLogStore((state) => state.lines);
  const clearLines = useBuildLogStore((state) => state.clear);

  const [sourceId, setSourceId] = useState<string | null>(null);
  const [selectedProfile, setProfile] = useState<BuildProfile | null>(null);
  const profile = selectedProfile ?? {
    ...defaultBuildProfile,
    backend: toolchain.data?.defaultBackend ?? "cpu",
  };
  const [clean, setClean] = useState(false);

  const availableSources = sources.data ?? [];
  const selectedSourceId =
    availableSources.find((source) => source.id === sourceId)?.id ??
    availableSources[0]?.id ??
    null;

  const cudaReady =
    toolchain.data?.tools
      .filter((tool) => tool.requirement === "requiredForCuda")
      .every((tool) => tool.found) ?? false;
  const baseReady =
    toolchain.data?.tools
      .filter((tool) => tool.requirement === "required")
      .every((tool) => tool.found) ?? false;

  const blocked =
    !baseReady || !toolchain.data?.backends.includes(profile.backend) || (profile.backend === "cuda" && !cudaReady) || !selectedSourceId;
  const isBuilding = buildRuntime.isPending || buildStatus.data === true;

  const start = () => {
    if (!selectedSourceId) {
      return;
    }
    clearLines();
    const channel = createProgressChannel((event) => {
      useBuildLogStore.getState().append(event);
    });
    buildRuntime.mutate({
      request: { sourceId: selectedSourceId, profile, clean },
      channel,
    });
  };

  return (
    <Section
      label="Build"
      title="Compile llama-server"
      description="A successful build is copied into a new immutable runtime; existing runtimes are never touched."
      icon={HammerIcon}
      bodyClassName="flex flex-col gap-6 p-4"
      actions={
        isBuilding ? (
          <Button
            variant="destructive"
            size="sm"
            disabled={cancelBuild.isPending}
            onClick={() => cancelBuild.mutate()}
          >
            <SquareIcon data-icon="inline-start" />
            Cancel
          </Button>
        ) : (
          <Button size="sm" disabled={blocked} onClick={start}>
            <HammerIcon data-icon="inline-start" />
            Configure &amp; build
          </Button>
        )
      }
    >
      {!baseReady && toolchain.data ? (
        <Alert variant="destructive">
          <SquareIcon />
          <AlertTitle>The build environment is incomplete</AlertTitle>
          <AlertDescription>
            Install the missing required tools listed above, then re-detect.
          </AlertDescription>
        </Alert>
      ) : null}

      {profile.backend === "cuda" && baseReady && !cudaReady && toolchain.data ? (
        <Alert variant="destructive">
          <MicrochipIcon />
          <AlertTitle>CUDA is selected but its toolchain is missing</AlertTitle>
          <AlertDescription>
            Install the CUDA Toolkit, or switch the backend to CPU to build without a GPU.
          </AlertDescription>
        </Alert>
      ) : null}

      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="build-source">Source</FieldLabel>
          <Select
            value={selectedSourceId ?? undefined}
            onValueChange={(value) => setSourceId(value)}
          >
            <SelectTrigger id="build-source">
              <SelectValue placeholder="No llama.cpp source registered" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {availableSources.map((source) => (
                  <SelectItem key={source.id} value={source.id}>
                    {source.name} · {source.currentRef} ·{" "}
                    {source.currentCommit.slice(0, 7)}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
          <FieldDescription>
            The commit currently checked out is the commit that gets built.
          </FieldDescription>
        </Field>

        <Field>
          <FieldLabel>Backend</FieldLabel>
          <ToggleGroup
            type="single"
            variant="outline"
            spacing={0}
            value={profile.backend}
            onValueChange={(value) => {
              const parsed = buildBackendSchema.safeParse(value);
              if (parsed.success) {
                setProfile({ ...profile, backend: parsed.data });
              }
            }}
          >
            {(toolchain.data?.backends ?? ["cpu"]).map((backend) => (
              <ToggleGroupItem key={backend} value={backend}>
                {backend === "cpu" ? <CpuIcon /> : <MicrochipIcon />}
                {backendLabel(backend)}
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
          <FieldDescription>
            CPU uses the processor; Metal accelerates Apple Silicon; CUDA uses NVIDIA GPUs.
          </FieldDescription>
        </Field>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field>
            <FieldLabel htmlFor="build-configuration">Configuration</FieldLabel>
            <Select
              value={profile.configuration}
              onValueChange={(value) => {
                const parsed = buildConfigurationSchema.safeParse(value);
                if (parsed.success) {
                  setProfile({ ...profile, configuration: parsed.data });
                }
              }}
            >
              <SelectTrigger id="build-configuration">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {(["release", "relWithDebInfo", "debug"] as const).map((value) => (
                    <SelectItem key={value} value={value}>
                      {configurationLabel(value)}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>

          <Field>
            <FieldLabel htmlFor="build-generator">Generator</FieldLabel>
            <Select
              value={profile.generator ?? CMAKE_DEFAULT_GENERATOR}
              onValueChange={(value) =>
                setProfile({
                  ...profile,
                  generator: value === CMAKE_DEFAULT_GENERATOR ? null : value,
                })
              }
            >
              <SelectTrigger id="build-generator">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value={CMAKE_DEFAULT_GENERATOR}>
                    Auto (CMake default)
                  </SelectItem>
                  {(toolchain.data?.generators ?? []).map((generator) => (
                    <SelectItem key={generator.name} value={generator.name}>
                      {generator.name}
                      {generator.isDefault ? " · default" : ""}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <FieldDescription>Reported by CMake on this machine.</FieldDescription>
          </Field>
        </div>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field>
            <FieldLabel htmlFor="build-jobs">Parallel jobs</FieldLabel>
            <Input
              id="build-jobs"
              type="number"
              min={1}
              max={256}
              value={profile.parallelJobs ?? ""}
              placeholder={
                settings.data?.build.parallelJobs
                  ? `Settings default: ${settings.data.build.parallelJobs}`
                  : "Let CMake decide"
              }
              onChange={(event) => {
                const raw = event.currentTarget.value;
                setProfile({
                  ...profile,
                  parallelJobs: raw === "" ? null : Number(raw),
                });
              }}
            />
            <FieldDescription>
              Empty uses the global Settings value, then falls back to CMake. Lower this if the
              compiler runs out of memory.
            </FieldDescription>
          </Field>

          <Field>
            <FieldLabel htmlFor="build-cuda-arch">CUDA architectures</FieldLabel>
            <Input
              id="build-cuda-arch"
              value={profile.cudaArchitectures ?? ""}
              placeholder="Detected by nvcc"
              disabled={profile.backend !== "cuda"}
              onChange={(event) =>
                setProfile({
                  ...profile,
                  cudaArchitectures: event.currentTarget.value || null,
                })
              }
            />
            <FieldDescription>
              Only needed when nvcc cannot detect the GPU, e.g. <code>86;89</code>.
            </FieldDescription>
          </Field>
        </div>

        <Field orientation="horizontal">
          <Switch
            id="build-native"
            checked={profile.nativeOptimizations}
            onCheckedChange={(checked) =>
              setProfile({ ...profile, nativeOptimizations: checked })
            }
          />
          <FieldLabel htmlFor="build-native">
            Optimize for this machine (upstream default; turn off for a portable binary)
          </FieldLabel>
        </Field>

        <Field orientation="horizontal">
          <Switch id="build-clean" checked={clean} onCheckedChange={setClean} />
          <FieldLabel htmlFor="build-clean">
            Clean build (delete the build tree first — slow, but fixes a stale cache)
          </FieldLabel>
        </Field>

        <Field>
          <FieldLabel htmlFor="build-extra">Additional CMake arguments</FieldLabel>
          <Textarea
            id="build-extra"
            value={profile.additionalCmakeArgs.join("\n")}
            placeholder={"-DGGML_NATIVE=OFF"}
            onChange={(event) =>
              setProfile({
                ...profile,
                additionalCmakeArgs: event.currentTarget.value
                  .split(/\r?\n/)
                  .map((argument) => argument.trim())
                  .filter(Boolean),
              })
            }
          />
          <FieldDescription>
            Enter one argument per line. Spaces inside a line stay in that single argument; no
            shell string is constructed.
          </FieldDescription>
        </Field>
      </FieldGroup>

      {isBuilding ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Spinner />
          Building {backendLabel(profile.backend)} · this takes several minutes
        </div>
      ) : null}

      {lines.length > 0 ? <OutputConsole lines={lines} className="h-72" /> : null}
    </Section>
  );
}
