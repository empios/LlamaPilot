import {
  CpuIcon,
  LoaderCircleIcon,
  RefreshCwIcon,
  SlidersHorizontalIcon,
  TerminalIcon,
} from "lucide-react";
import { useEffect } from "react";

import { ErrorPanel } from "@/components/error-panel";
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
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  useInspectRuntimeCapabilities,
  useRuntimeCapabilities,
} from "@/hooks/use-build";
import { formatTimestamp } from "@/lib/format";
import type { RuntimeRecord } from "@/types/build";
import type {
  LlamaOption,
  RawCommandOutput,
  RuntimeInspection,
} from "@/types/capabilities";

interface RuntimeCapabilitiesDialogProps {
  runtime: RuntimeRecord | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function RuntimeCapabilitiesDialog({
  runtime,
  open,
  onOpenChange,
}: RuntimeCapabilitiesDialogProps) {
  const existing = useRuntimeCapabilities(
    runtime?.id ?? null,
    open && runtime?.capabilities !== null,
  );
  const inspect = useInspectRuntimeCapabilities();
  const inspection = existing.data;

  useEffect(() => {
    inspect.reset();
  }, [runtime?.id]);

  if (!runtime) {
    return null;
  }

  const runInspection = () => inspect.mutate(runtime.id);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-hidden sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>
            Runtime capabilities · {runtime.branch} @ {runtime.shortCommit}
          </DialogTitle>
          <DialogDescription>
            Read directly from this copied llama-server binary. Re-inspection never
            changes the executable or its libraries.
          </DialogDescription>
        </DialogHeader>

        {runtime.capabilities === null && !inspection ? (
          <UninspectedState pending={inspect.isPending} onInspect={runInspection} />
        ) : null}

        {runtime.capabilities !== null && existing.isPending ? (
          <div className="grid gap-3 py-2">
            <Skeleton className="h-20 w-full" />
            <Skeleton className="h-72 w-full" />
          </div>
        ) : null}

        {inspect.isError ? <ErrorPanel error={inspect.error} /> : null}

        {existing.isError ? (
          <div className="flex flex-col gap-3 py-2">
            <ErrorPanel error={existing.error} />
            <Button
              className="self-start"
              disabled={inspect.isPending}
              onClick={runInspection}
            >
              {inspect.isPending ? (
                <LoaderCircleIcon className="animate-spin" data-icon="inline-start" />
              ) : (
                <RefreshCwIcon data-icon="inline-start" />
              )}
              Inspect again
            </Button>
          </div>
        ) : null}

        {inspection ? <InspectionView inspection={inspection} /> : null}

        {inspection ? (
          <DialogFooter>
            <span className="mr-auto self-center text-xs text-muted-foreground">
              {runtime.capabilities
                ? `Last inspected ${formatTimestamp(runtime.capabilities.inspectedAt)}`
                : "Inspection completed just now"}
            </span>
            <Button
              variant="outline"
              disabled={inspect.isPending}
              onClick={runInspection}
            >
              {inspect.isPending ? (
                <LoaderCircleIcon className="animate-spin" data-icon="inline-start" />
              ) : (
                <RefreshCwIcon data-icon="inline-start" />
              )}
              Inspect again
            </Button>
          </DialogFooter>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

function UninspectedState({
  pending,
  onInspect,
}: {
  pending: boolean;
  onInspect: () => void;
}) {
  return (
    <div className="flex min-h-72 flex-col items-center justify-center gap-4 rounded-lg border border-dashed p-8 text-center">
      <span className="flex size-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
        <SlidersHorizontalIcon className="size-5" />
      </span>
      <div className="max-w-md space-y-1">
        <p className="font-medium">This older runtime has not been inspected</p>
        <p className="text-sm text-muted-foreground">
          Llama Control will ask it for its version, supported options, and
          available devices, then keep the original output with the runtime.
        </p>
      </div>
      <Button disabled={pending} onClick={onInspect}>
        {pending ? (
          <LoaderCircleIcon className="animate-spin" data-icon="inline-start" />
        ) : (
          <SlidersHorizontalIcon data-icon="inline-start" />
        )}
        Inspect runtime
      </Button>
    </div>
  );
}

function InspectionView({ inspection }: { inspection: RuntimeInspection }) {
  const { capabilities, raw } = inspection;
  const options = Object.values(capabilities.options);
  const known = options
    .filter((option) => option.knownKey !== null)
    .sort(compareOptions);
  const advanced = options
    .filter((option) => option.knownKey === null)
    .sort(compareOptions);

  return (
    <div className="min-h-0 overflow-hidden">
      <div className="mb-4 grid gap-3 sm:grid-cols-3">
        <Metric label="Version" value={capabilities.version} />
        <Metric label="Options" value={`${known.length} known · ${advanced.length} advanced`} />
        <Metric
          label="Devices"
          value={capabilities.devices.length > 0 ? String(capabilities.devices.length) : "None"}
        />
      </div>

      <Tabs defaultValue="overview">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="known">Known ({known.length})</TabsTrigger>
          <TabsTrigger value="advanced">Advanced ({advanced.length})</TabsTrigger>
          <TabsTrigger value="raw">Raw output</TabsTrigger>
        </TabsList>

        <ScrollArea className="mt-2 h-[48vh] rounded-lg border">
          <TabsContent value="overview" className="p-4">
            <Overview inspection={inspection} />
          </TabsContent>
          <TabsContent value="known" className="p-2">
            <OptionList
              options={known}
              empty="No important options were recognised in this runtime."
            />
          </TabsContent>
          <TabsContent value="advanced" className="p-2">
            <OptionList
              options={advanced}
              empty="This runtime advertises no additional options."
            />
          </TabsContent>
          <TabsContent value="raw" className="space-y-4 p-4">
            <RawOutput label="--version" output={raw.version} />
            <RawOutput label="--help" output={raw.help} />
            <RawOutput label="--list-devices" output={raw.devices} />
          </TabsContent>
        </ScrollArea>
      </Tabs>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-lg border bg-muted/30 px-3 py-2.5">
      <p className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
        {label}
      </p>
      <p className="truncate pt-0.5 text-sm font-medium" title={value}>
        {value}
      </p>
    </div>
  );
}

function Overview({ inspection }: { inspection: RuntimeInspection }) {
  const { capabilities } = inspection;
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h3 className="flex items-center gap-2 text-sm font-medium">
          <CpuIcon className="size-4 text-muted-foreground" />
          Available devices
        </h3>
        {capabilities.devices.length > 0 ? (
          <ul className="divide-y rounded-md border">
            {capabilities.devices.map((device) => (
              <li key={device.id} className="flex items-center justify-between gap-4 px-3 py-2.5">
                <span className="min-w-0">
                  <code className="font-mono text-xs">{device.id}</code>
                  <span className="ml-2 text-sm">{device.name}</span>
                </span>
                {device.memoryTotalMib !== null ? (
                  <Badge variant="outline" className="shrink-0">
                    {formatMib(device.memoryTotalMib)}
                    {device.memoryFreeMib !== null
                      ? ` · ${formatMib(device.memoryFreeMib)} free`
                      : ""}
                  </Badge>
                ) : null}
              </li>
            ))}
          </ul>
        ) : (
          <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">
            This runtime reported no accelerator devices.
          </p>
        )}
      </div>

      <div className="space-y-2">
        <h3 className="flex items-center gap-2 text-sm font-medium">
          <SlidersHorizontalIcon className="size-4 text-muted-foreground" />
          Speculative decoding types
        </h3>
        {capabilities.speculativeTypes.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            {capabilities.speculativeTypes.map((type) => (
              <Badge key={type} variant="secondary" className="font-mono font-normal">
                {type}
              </Badge>
            ))}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            The runtime does not advertise <code>--spec-type</code> values.
          </p>
        )}
      </div>
    </div>
  );
}

function OptionList({ options, empty }: { options: LlamaOption[]; empty: string }) {
  if (options.length === 0) {
    return <p className="p-3 text-sm text-muted-foreground">{empty}</p>;
  }

  return (
    <ul className="divide-y">
      {options.map((option) => (
        <li key={option.flag} className="space-y-1 px-2 py-3">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium">{option.displayName}</span>
            <code className="font-mono text-xs text-primary">{option.flag}</code>
            {option.valueHint ? (
              <Badge variant="outline" className="font-mono font-normal">
                {option.valueHint}
              </Badge>
            ) : null}
          </div>
          <p className="text-xs text-muted-foreground">
            {option.summary ?? (option.description || "No description reported.")}
          </p>
          {option.aliases.length > 1 ? (
            <p className="font-mono text-[11px] text-muted-foreground/80">
              {option.aliases.join(" · ")}
            </p>
          ) : null}
        </li>
      ))}
    </ul>
  );
}

function RawOutput({ label, output }: { label: string; output: RawCommandOutput }) {
  return (
    <section className="space-y-2">
      <h3 className="flex items-center gap-2 text-sm font-medium">
        <TerminalIcon className="size-4 text-muted-foreground" />
        llama-server {label}
      </h3>
      <pre className="max-h-72 overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap text-muted-foreground">
        {renderRaw(output)}
      </pre>
    </section>
  );
}

function renderRaw(output: RawCommandOutput): string {
  const streams = [];
  if (output.stdout.trim()) {
    streams.push(`[stdout]\n${output.stdout.trimEnd()}`);
  }
  if (output.stderr.trim()) {
    streams.push(`[stderr]\n${output.stderr.trimEnd()}`);
  }
  return streams.join("\n\n") || "(no output)";
}

function compareOptions(left: LlamaOption, right: LlamaOption): number {
  return left.displayName.localeCompare(right.displayName);
}

function formatMib(value: number): string {
  return value >= 1024 ? `${(value / 1024).toFixed(1)} GiB` : `${value} MiB`;
}
