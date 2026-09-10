import {
  AlertTriangleIcon,
  CheckCircle2Icon,
  GaugeIcon,
  PlayIcon,
  RocketIcon,
  SaveIcon,
  SquareIcon,
  TrophyIcon,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { ErrorPanel } from "@/components/error-panel";
import { PageHeader } from "@/components/page-header";
import { Section } from "@/components/section";
import { StatGrid, StatTile } from "@/components/stat-tile";
import { StatusChip } from "@/components/status-chip";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Progress } from "@/components/ui/progress";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useHardwareSnapshot } from "@/hooks/use-app-info";
import {
  usePerformanceBenchmarks,
  usePerformancePlan,
  usePerformanceSweepStatus,
  useCancelPerformanceSweep,
  useRunPerformanceBenchmark,
  useRunPerformanceSweep,
} from "@/hooks/use-performance";
import { useProfiles, useUpdateProfile } from "@/hooks/use-profiles";
import { useServerStatus, useStartServer } from "@/hooks/use-server";
import { formatBytes, formatMebibytes, formatTimestamp } from "@/lib/format";
import { createPerformanceSweepChannel } from "@/lib/ipc";
import type {
  PerformanceCandidate,
  PerformanceSweep,
  PerformanceSweepEvent,
} from "@/types/performance";
import {
  profileToInput,
  type LaunchProfile,
  type ProfileInput,
} from "@/types/profiles";
import { serverIsActive } from "@/types/server";

export function PerformancePage() {
  const profiles = useProfiles();
  const hardware = useHardwareSnapshot();
  const server = useServerStatus();
  const startServer = useStartServer();
  const updateProfile = useUpdateProfile();
  const runBenchmark = useRunPerformanceBenchmark();
  const sweepStatus = usePerformanceSweepStatus();
  const runSweep = useRunPerformanceSweep();
  const cancelSweep = useCancelPerformanceSweep();
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const plan = usePerformancePlan(selectedProfileId);
  const benchmarks = usePerformanceBenchmarks(selectedProfileId);
  const [selectedCandidateId, setSelectedCandidateId] = useState<string | null>(null);
  const [sweepEvent, setSweepEvent] = useState<PerformanceSweepEvent | null>(null);

  useEffect(() => {
    if (!selectedProfileId && profiles.data?.[0]) {
      setSelectedProfileId(profiles.data[0].id);
    }
  }, [profiles.data, selectedProfileId]);

  useEffect(() => {
    if (plan.data?.recommendedCandidateId) {
      setSelectedCandidateId((current) =>
        plan.data?.candidates.some((candidate) => candidate.id === current)
          ? current
          : plan.data?.recommendedCandidateId ?? null,
      );
    } else {
      setSelectedCandidateId(null);
    }
  }, [plan.data]);

  const selectedProfile = profiles.data?.find((profile) => profile.id === selectedProfileId) ?? null;
  const selectedCandidate =
    plan.data?.candidates.find((candidate) => candidate.id === selectedCandidateId) ?? null;
  const serverActive = server.data ? serverIsActive(server.data.state) : false;
  const selectedServerActive = serverActive && server.data?.profileId === selectedProfileId;
  const readyToBenchmark = selectedServerActive && server.data?.state === "ready";
  const sweepActive = runSweep.isPending || sweepStatus.data === true;
  const latest = benchmarks.data?.[0] ?? null;
  const bestGeneration = useMemo(
    () =>
      benchmarks.data?.reduce((best, result) =>
        !best || result.predictedTokensPerSecond > best.predictedTokensPerSecond ? result : best,
      undefined as (typeof benchmarks.data)[number] | undefined),
    [benchmarks.data],
  );
  const firstError =
    profiles.error ??
    hardware.error ??
    server.error ??
    plan.error ??
    benchmarks.error ??
    sweepStatus.error;

  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="Serve"
        title="Performance Lab"
        description="Place one coding model across every GPU, apply runtime-valid candidates, and compare repeatable llama-server timings."
      />

      {profiles.isPending || hardware.isPending ? <Skeleton className="h-44 w-full" /> : null}
      {firstError ? <ErrorPanel error={firstError} /> : null}

      {!profiles.isPending && profiles.data?.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <RocketIcon />
            </EmptyMedia>
            <EmptyTitle>Create a launch profile first</EmptyTitle>
            <EmptyDescription>
              Performance Lab keeps the model, runtime, and all non-placement settings pinned to a saved profile.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : null}

      {profiles.data && profiles.data.length > 0 ? (
        <>
          <div className="flex flex-wrap gap-2">
            <StatusChip label="Detected GPUs" value={String(hardware.data?.gpus.length ?? 0)} />
            <StatusChip
              label="Free VRAM"
              value={formatMebibytes(
                hardware.data?.gpus.reduce((total, gpu) => total + gpu.freeMemoryMib, 0) ?? 0,
              )}
            />
            <StatusChip label="Saved runs" value={String(benchmarks.data?.length ?? 0)} />
            <StatusChip label="Objective" value="Interactive coding" />
          </div>

          <Section
            label="Target"
            title="Profile under test"
            description="The lab never changes model or runtime identity."
            icon={GaugeIcon}
          >
            <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
              <label className="flex flex-col gap-2 text-sm font-medium" htmlFor="performance-profile">
                Launch profile
                <Select value={selectedProfileId ?? undefined} onValueChange={setSelectedProfileId}>
                  <SelectTrigger id="performance-profile" className="w-full">
                    <SelectValue placeholder="Choose a profile" />
                  </SelectTrigger>
                  <SelectContent>
                    {profiles.data.map((profile) => (
                      <SelectItem key={profile.id} value={profile.id}>
                        {profile.name} · {profile.modelName}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </label>
              {selectedProfile ? (
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="secondary">{selectedProfile.modelName}</Badge>
                  <Badge variant="outline">{selectedProfile.runtimeLabel}</Badge>
                </div>
              ) : null}
            </div>
          </Section>

          {plan.isPending ? <Skeleton className="h-80 w-full" /> : null}

          {plan.data ? (
            <>
              <Section
                label="Placement plan"
                title={`${plan.data.devices.length} GPU configuration`}
                description="Initial proportions reserve memory for the desktop, KV cache, and runtime overhead."
                icon={RocketIcon}
                bodyClassName="flex flex-col gap-6 p-4"
              >
                {plan.data.warnings.map((warning) => (
                  <Alert key={warning}>
                    <AlertTriangleIcon />
                    <AlertTitle>Plan warning</AlertTitle>
                    <AlertDescription>{warning}</AlertDescription>
                  </Alert>
                ))}

                {plan.data.devices.length > 0 ? (
                  <div className="grid gap-4 lg:grid-cols-2">
                    {plan.data.devices.map((device) => (
                      <div key={device.id} className="rounded-md border border-border p-4">
                        <div className="flex flex-col gap-1.5">
                          <div className="flex items-baseline justify-between gap-4">
                            <span className="truncate text-sm font-medium">
                              {device.id} · {device.name}
                            </span>
                            <span className="shrink-0 font-mono text-xs tabular-nums text-foreground">
                              {device.splitPercent}%
                            </span>
                          </div>
                          <div
                            role="meter"
                            aria-label={`${device.id} model allocation`}
                            aria-valuemin={0}
                            aria-valuemax={100}
                            aria-valuenow={device.splitPercent}
                            className="h-2 overflow-hidden rounded-full bg-muted"
                          >
                            <div
                              className="h-full rounded-full bg-primary transition-[width]"
                              style={{ width: `${device.splitPercent}%` }}
                            />
                          </div>
                          <span className="text-xs text-muted-foreground">
                            {device.freeMemoryMib !== null
                              ? `${formatMebibytes(device.freeMemoryMib)} free${
                                  device.totalMemoryMib !== null
                                    ? ` of ${formatMebibytes(device.totalMemoryMib)}`
                                    : ""
                                }`
                              : "Runtime did not report free memory"}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : null}

                <StatGrid className="lg:grid-cols-3">
                  <StatTile
                    label="Model files"
                    value={
                      plan.data.modelSizeBytes !== null
                        ? formatBytes(plan.data.modelSizeBytes)
                        : "Unknown"
                    }
                    detail={plan.data.modelName}
                  />
                  <StatTile
                    label="Free VRAM"
                    value={formatMebibytes(plan.data.totalFreeMemoryMib)}
                    detail="Point-in-time value before launch"
                  />
                  <StatTile
                    label="Candidates"
                    value={String(plan.data.candidates.length)}
                    detail="Generated from this runtime's --help and --list-devices"
                  />
                </StatGrid>

                {plan.data.candidates.length > 0 ? (
                  <div className="grid gap-3 lg:grid-cols-3">
                    {plan.data.candidates.map((candidate) => {
                      const selected = candidate.id === selectedCandidateId;
                      const recommended = candidate.id === plan.data?.recommendedCandidateId;
                      return (
                        <button
                          key={candidate.id}
                          type="button"
                          aria-pressed={selected}
                          onClick={() => setSelectedCandidateId(candidate.id)}
                          className={`flex min-w-0 flex-col gap-3 rounded-lg border p-4 text-left transition-colors ${
                            selected
                              ? "border-primary bg-primary/5"
                              : "border-border hover:bg-muted/40"
                          }`}
                        >
                          <span className="flex flex-wrap items-center gap-2">
                            <span className="font-semibold">{candidate.label}</span>
                            {recommended ? <Badge>Baseline</Badge> : null}
                            {candidate.experimental ? <Badge variant="outline">Experimental</Badge> : null}
                          </span>
                          <span className="text-xs leading-relaxed text-muted-foreground">
                            {candidate.description}
                          </span>
                          <span className="font-mono text-xs text-foreground">
                            {candidate.options.tensorSplit?.mode === "custom"
                              ? `split ${candidate.options.tensorSplit.value}`
                              : candidate.splitMode}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                ) : null}

                {selectedCandidate ? (
                  <CandidateDetails
                    candidate={selectedCandidate}
                    recommended={selectedCandidate.id === plan.data.recommendedCandidateId}
                  />
                ) : null}

                <div className="flex justify-end">
                  <Button
                    disabled={!selectedProfile || !selectedCandidate || updateProfile.isPending || selectedServerActive}
                    title={selectedServerActive ? "Stop this profile before changing its placement" : undefined}
                    onClick={() => {
                      if (!selectedProfile || !selectedCandidate) return;
                      updateProfile.mutate({
                        id: selectedProfile.id,
                        input: applyCandidateOptions(selectedProfile, selectedCandidate),
                      });
                    }}
                  >
                    {updateProfile.isPending ? <Spinner /> : <SaveIcon data-icon="inline-start" />}
                    Apply to profile
                  </Button>
                </div>
              </Section>

              <Section
                label="Automatic tuning"
                title="Test every candidate and keep the winner"
                description="Performance Lab starts, measures, and stops each placement in isolation, then saves the fastest generation result to this profile."
                icon={TrophyIcon}
                actions={
                  sweepActive ? (
                    <Button
                      variant="outline"
                      disabled={cancelSweep.isPending}
                      onClick={() => cancelSweep.mutate()}
                    >
                      {cancelSweep.isPending ? (
                        <Spinner />
                      ) : (
                        <SquareIcon data-icon="inline-start" />
                      )}
                      Cancel sweep
                    </Button>
                  ) : (
                    <Button
                      disabled={
                        !selectedProfileId ||
                        serverActive ||
                        !plan.data.candidates.length
                      }
                      title={
                        serverActive
                          ? "Stop the active server before automatic tuning"
                          : undefined
                      }
                      onClick={() => {
                        if (!selectedProfileId) return;
                        runSweep.reset();
                        setSweepEvent(null);
                        runSweep.mutate({
                          profileId: selectedProfileId,
                          channel: createPerformanceSweepChannel(setSweepEvent),
                        });
                      }}
                    >
                      <RocketIcon data-icon="inline-start" />
                      Auto-tune {plan.data.candidates.length} candidate
                      {plan.data.candidates.length === 1 ? "" : "s"}
                    </Button>
                  )
                }
                bodyClassName="flex flex-col gap-5 p-4"
              >
                <Alert>
                  <GaugeIcon />
                  <AlertTitle>Comparable, unattended measurements</AlertTitle>
                  <AlertDescription>
                    Each candidate performs a fresh model load and one deterministic coding request.
                    The server is stopped afterward. Failed candidates are skipped; cancellation
                    restores the original profile.
                  </AlertDescription>
                </Alert>

                {sweepActive ? <SweepProgress event={sweepEvent} /> : null}
                {runSweep.data ? <SweepResults sweep={runSweep.data} /> : null}
              </Section>

              <Section
                label="Coding benchmark"
                title="Measure the active configuration"
                description="A deterministic long Rust prompt requests 128 output tokens with cache reuse disabled."
                icon={GaugeIcon}
                actions={
                  readyToBenchmark ? (
                    <Button
                      disabled={runBenchmark.isPending}
                      onClick={() => selectedProfileId && runBenchmark.mutate(selectedProfileId)}
                    >
                      {runBenchmark.isPending ? <Spinner /> : <GaugeIcon data-icon="inline-start" />}
                      Run benchmark
                    </Button>
                  ) : (
                    <Button
                      disabled={!selectedProfileId || serverActive || startServer.isPending}
                      title={serverActive ? "Stop the active server before starting this profile" : undefined}
                      onClick={() => selectedProfileId && startServer.mutate(selectedProfileId)}
                    >
                      {startServer.isPending ? <Spinner /> : <PlayIcon data-icon="inline-start" />}
                      Start selected profile
                    </Button>
                  )
                }
                bodyClassName="flex flex-col gap-6 p-4"
              >
                <BenchmarkReadiness
                  selectedServerActive={selectedServerActive}
                  ready={readyToBenchmark}
                  serverActive={serverActive}
                />

                {latest ? (
                  <StatGrid className="lg:grid-cols-4">
                    <StatTile
                      label="Prompt processing"
                      value={`${latest.promptTokensPerSecond.toFixed(1)} tok/s`}
                      detail={`${latest.promptTokens} measured tokens`}
                    />
                    <StatTile
                      label="Generation"
                      value={`${latest.predictedTokensPerSecond.toFixed(1)} tok/s`}
                      detail={`${latest.predictedTokens} generated tokens`}
                    />
                    <StatTile
                      label="End-to-end"
                      value={`${(latest.latencyMs / 1_000).toFixed(2)} s`}
                      detail={formatTimestamp(latest.createdAt)}
                    />
                    <StatTile
                      label="Best generation"
                      value={
                        bestGeneration
                          ? `${bestGeneration.predictedTokensPerSecond.toFixed(1)} tok/s`
                          : "—"
                      }
                      detail={bestGeneration?.placement.splitMode ?? "No placement recorded"}
                    />
                  </StatGrid>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    No benchmark has been recorded for this profile yet.
                  </p>
                )}

                {benchmarks.data && benchmarks.data.length > 0 ? (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Run</TableHead>
                        <TableHead>Placement</TableHead>
                        <TableHead className="text-right">Prompt tok/s</TableHead>
                        <TableHead className="text-right">Generation tok/s</TableHead>
                        <TableHead className="text-right">Latency</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {benchmarks.data.map((result) => (
                        <TableRow key={result.id}>
                          <TableCell>
                            <div className="flex flex-col">
                              <span className="font-medium">{formatTimestamp(result.createdAt)}</span>
                              <span className="text-xs text-muted-foreground">
                                {result.gpus.length} GPU{result.gpus.length === 1 ? "" : "s"}
                              </span>
                            </div>
                          </TableCell>
                          <TableCell className="font-mono text-xs">
                            {result.placement.splitMode ?? "default"}
                            {result.placement.tensorSplit
                              ? ` · ${result.placement.tensorSplit}`
                              : ""}
                          </TableCell>
                          <TableCell className="text-right font-mono tabular-nums">
                            {result.promptTokensPerSecond.toFixed(1)}
                          </TableCell>
                          <TableCell className="text-right font-mono tabular-nums">
                            {result.predictedTokensPerSecond.toFixed(1)}
                          </TableCell>
                          <TableCell className="text-right font-mono tabular-nums">
                            {(result.latencyMs / 1_000).toFixed(2)} s
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                ) : null}
              </Section>
            </>
          ) : null}
        </>
      ) : null}
    </div>
  );
}

function SweepProgress({ event }: { event: PerformanceSweepEvent | null }) {
  let label = "Preparing automatic benchmark";
  let detail = "Waiting for the first candidate to start.";
  let progress = 0;

  if (event?.kind === "started") {
    detail = `${event.totalCandidates} candidate${event.totalCandidates === 1 ? "" : "s"} queued.`;
  } else if (event?.kind === "candidateStarted") {
    label = `Testing ${event.candidateLabel}`;
    detail = `Candidate ${event.index} of ${event.totalCandidates} · loading the model and measuring generation.`;
    progress = ((event.index - 1) / event.totalCandidates) * 100;
  } else if (event?.kind === "candidateFinished") {
    label = event.result.success
      ? `${event.result.candidateLabel} measured`
      : `${event.result.candidateLabel} skipped`;
    detail = `${event.completedCandidates} of ${event.totalCandidates} candidates completed.`;
    progress = (event.completedCandidates / event.totalCandidates) * 100;
  } else if (event?.kind === "applyingWinner") {
    label = `Saving ${event.candidateLabel}`;
    detail = "All candidates are complete. Applying the fastest successful placement.";
    progress = 100;
  } else if (event?.kind === "finished") {
    label = event.success ? "Automatic tuning complete" : "Automatic tuning finished";
    detail = event.success
      ? "The winning placement is saved to the profile."
      : "No winner was applied.";
    progress = 100;
  }

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-primary/30 bg-primary/5 p-4">
      <div className="flex items-center gap-3">
        <Spinner />
        <div className="flex min-w-0 flex-col">
          <span className="font-medium">{label}</span>
          <span className="text-xs leading-relaxed text-muted-foreground">{detail}</span>
        </div>
      </div>
      <Progress value={progress} aria-label="Automatic tuning progress" />
    </div>
  );
}

function SweepResults({ sweep }: { sweep: PerformanceSweep }) {
  const winner = sweep.results.find(
    (result) => result.candidateId === sweep.winnerCandidateId,
  );
  return (
    <div className="flex flex-col gap-4">
      {winner?.benchmark ? (
        <div className="grid gap-4 rounded-lg border border-primary/40 bg-primary/5 p-4 lg:grid-cols-[1fr_auto] lg:items-center">
          <div className="flex flex-col gap-1">
            <span className="flex items-center gap-2 font-semibold">
              <TrophyIcon className="size-4 text-primary" />
              {winner.candidateLabel}
            </span>
            <span className="text-xs text-muted-foreground">
              Winning placement saved to {sweep.profileName}
            </span>
          </div>
          <div className="flex gap-6 font-mono text-sm tabular-nums">
            <span>{winner.benchmark.promptTokensPerSecond.toFixed(1)} prompt tok/s</span>
            <span>{winner.benchmark.predictedTokensPerSecond.toFixed(1)} generation tok/s</span>
          </div>
        </div>
      ) : (
        <Alert>
          <AlertTriangleIcon />
          <AlertTitle>No candidate completed</AlertTitle>
          <AlertDescription>The original profile settings were restored.</AlertDescription>
        </Alert>
      )}

      <div className="grid gap-3 lg:grid-cols-3">
        {sweep.results.map((result) => (
          <div key={result.candidateId} className="flex flex-col gap-2 rounded-md border p-3">
            <span className="flex items-center justify-between gap-2">
              <span className="truncate text-sm font-medium">{result.candidateLabel}</span>
              <Badge variant={result.success ? "secondary" : "outline"}>
                {result.success ? "Measured" : "Failed"}
              </Badge>
            </span>
            {result.benchmark ? (
              <span className="font-mono text-xs tabular-nums text-muted-foreground">
                {result.benchmark.predictedTokensPerSecond.toFixed(1)} generation tok/s
              </span>
            ) : (
              <span className="text-xs leading-relaxed text-muted-foreground">
                {result.error ?? "No measurement returned."}
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function CandidateDetails({
  candidate,
  recommended,
}: {
  candidate: PerformanceCandidate;
  recommended: boolean;
}) {
  return (
    <div className="grid gap-4 rounded-lg border border-border bg-muted/20 p-4 lg:grid-cols-2">
      <div className="flex flex-col gap-2">
        <span className="flex items-center gap-2 text-sm font-semibold">
          <CheckCircle2Icon className="size-4 text-primary" />
          Why this candidate{recommended ? " is the baseline" : " exists"}
        </span>
        <ul className="list-disc space-y-1 pl-5 text-xs leading-relaxed text-muted-foreground">
          {candidate.reasons.map((reason) => (
            <li key={reason}>{reason}</li>
          ))}
        </ul>
      </div>
      <div className="flex flex-col gap-2">
        <span className="text-sm font-semibold">Profile overrides</span>
        <div className="flex flex-wrap gap-1.5">
          {Object.entries(candidate.options).map(([key, setting]) => (
            <Badge key={key} variant="outline" className="font-mono font-normal">
              {key}={setting.mode === "custom" ? setting.value : setting.mode}
            </Badge>
          ))}
        </div>
        {candidate.warnings.map((warning) => (
          <span key={warning} className="text-xs leading-relaxed text-amber-700 dark:text-amber-300">
            {warning}
          </span>
        ))}
      </div>
    </div>
  );
}

function BenchmarkReadiness({
  selectedServerActive,
  ready,
  serverActive,
}: {
  selectedServerActive: boolean;
  ready: boolean;
  serverActive: boolean;
}) {
  if (ready) {
    return (
      <Alert>
        <CheckCircle2Icon />
        <AlertTitle>Ready for a clean measurement</AlertTitle>
        <AlertDescription>
          Keep coding agents idle until the request finishes. Their traffic would share the same slots and distort the result.
        </AlertDescription>
      </Alert>
    );
  }
  return (
    <Alert>
      <AlertTriangleIcon />
      <AlertTitle>
        {serverActive && !selectedServerActive
          ? "Another profile is active"
          : selectedServerActive
            ? "Waiting for Ready"
            : "Start this profile to benchmark it"}
      </AlertTitle>
      <AlertDescription>
        Performance Lab only measures an idle, healthy server using the currently selected profile.
      </AlertDescription>
    </Alert>
  );
}

export function applyCandidateOptions(
  profile: LaunchProfile,
  candidate: PerformanceCandidate,
): ProfileInput {
  const input = profileToInput(profile);
  const options = { ...input.options };
  for (const key of PERFORMANCE_MANAGED_OPTION_KEYS) delete options[key];
  for (const [key, setting] of Object.entries(candidate.options)) {
    if (setting.mode === "default") delete options[key];
    else options[key] = setting;
  }
  return { ...input, options };
}

const PERFORMANCE_MANAGED_OPTION_KEYS = [
  "device",
  "gpuLayers",
  "tensorSplit",
  "splitMode",
  "mainGpu",
  "flashAttention",
  "fit",
  "kvCacheOffload",
  "kvCacheTypeK",
  "kvCacheTypeV",
];
