import {
  AlertTriangleIcon,
  BotIcon,
  FolderOpenIcon,
  ImageIcon,
  LayersIcon,
  PackageIcon,
  RefreshCwIcon,
  SettingsIcon,
} from "lucide-react";

import { ErrorPanel } from "@/components/error-panel";
import { PageHeader } from "@/components/page-header";
import { Section } from "@/components/section";
import { StatusChip } from "@/components/status-chip";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
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
import { useGgufPicker } from "@/hooks/use-gguf-picker";
import { useModels, useSetModelProjector } from "@/hooks/use-models";
import { formatBytes, formatTimestamp, pluralize } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useNavigationStore } from "@/stores/navigation-store";
import {
  primaryModels,
  type ModelRecord,
  type ProjectorRecord,
} from "@/types/models";

const AUTOMATIC_VALUE = "__automatic__";
const DISABLED_VALUE = "__disabled__";
const MISSING_VALUE = "__missing__";

export function ModelsPage() {
  const catalog = useModels();
  const setProjector = useSetModelProjector();
  const pickGguf = useGgufPicker();
  const navigate = useNavigationStore((state) => state.navigate);

  const roots = catalog.data?.roots.length ?? 0;
  const models = catalog.data ? primaryModels(catalog.data.models) : [];
  const drafters = catalog.data?.models.filter((model) => model.role === "drafter") ?? [];
  const modelCount = models.length;
  const drafterCount = drafters.length;
  const projectorCount = catalog.data?.projectors.length ?? 0;

  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="Serve"
        title="Models"
        description="GGUF models discovered recursively without loading tensor data."
        actions={
          <>
            <Button variant="outline" onClick={() => navigate("settings")}>
              <SettingsIcon data-icon="inline-start" />
              Folders
            </Button>
            <Button disabled={catalog.isFetching} onClick={() => void catalog.refetch()}>
              <RefreshCwIcon
                data-icon="inline-start"
                className={cn(catalog.isFetching && "animate-spin")}
              />
              Scan now
            </Button>
          </>
        }
      />

      {catalog.isPending ? (
        <div className="flex flex-col gap-3">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-40 w-full" />
        </div>
      ) : null}
      {catalog.isError ? <ErrorPanel error={catalog.error} /> : null}

      {catalog.data ? (
        <div className="flex flex-wrap gap-2">
          <StatusChip label="Folders" value={String(roots)} />
          <StatusChip label="Models" value={String(modelCount)} />
          <StatusChip label="Drafters" value={String(drafterCount)} />
          <StatusChip label="Projectors" value={String(projectorCount)} />
          <StatusChip
            label="Metadata cache"
            value={`${catalog.data.cacheHits} hit / ${catalog.data.cacheMisses} read`}
            tone={catalog.data.cacheHits > 0 ? "success" : "neutral"}
          />
          <StatusChip label="Scanned" value={formatTimestamp(catalog.data.scannedAt)} />
        </div>
      ) : null}

      {catalog.data && catalog.data.issues.length > 0 ? (
        <Section
          label="Scan report"
          title={`${catalog.data.issues.length} ${pluralize(catalog.data.issues.length, "file or folder needs attention", "files or folders need attention")}`}
          description="Other valid models remain available."
          icon={AlertTriangleIcon}
          bodyClassName="p-0"
        >
          <ul className="max-h-56 divide-y divide-border overflow-auto">
            {catalog.data.issues.map((issue, index) => (
              <li key={`${issue.path}-${index}`} className="px-4 py-3">
                <p className="text-sm font-medium">{issue.message}</p>
                <code className="block truncate font-mono text-xs text-muted-foreground">
                  {issue.path}
                </code>
              </li>
            ))}
          </ul>
        </Section>
      ) : null}

      {catalog.data && roots === 0 ? (
        <Section label="Catalog" title="No model folders configured" icon={PackageIcon}>
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <FolderOpenIcon />
              </EmptyMedia>
              <EmptyTitle>Add a folder containing GGUF files</EmptyTitle>
              <EmptyDescription>
                Llama Control will scan it recursively and keep parsed metadata in a small cache.
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button onClick={() => navigate("settings")}>Configure model folders</Button>
            </EmptyContent>
          </Empty>
        </Section>
      ) : null}

      {catalog.data && roots > 0 ? (
        <Section
          label="Catalog"
          title={`${modelCount} ${pluralize(modelCount, "logical model")}`}
          description="Split files are grouped; the first shard is the launch path."
          icon={PackageIcon}
          bodyClassName={modelCount > 0 ? "p-0" : "p-4"}
        >
          {modelCount === 0 ? (
            <Empty>
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <PackageIcon />
                </EmptyMedia>
                <EmptyTitle>No primary model GGUF files found</EmptyTitle>
                <EmptyDescription>
                  The configured folders were scanned. Drafters and projectors are listed separately.
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          ) : (
            <ul className="divide-y divide-border">
              {models.map((model) => (
                <ModelRow
                  key={model.id}
                  model={model}
                  projectors={catalog.data.projectors}
                  pending={setProjector.isPending}
                  onSelection={(selection) =>
                    setProjector.mutate({ modelId: model.id, selection })
                  }
                  onBrowse={async () => {
                    const path = await pickGguf();
                    if (path) {
                      setProjector.mutate({
                        modelId: model.id,
                        selection: { mode: "custom", path },
                      });
                    }
                  }}
                />
              ))}
            </ul>
          )}
        </Section>
      ) : null}

      {catalog.data && drafterCount > 0 ? (
        <Section
          label="Speculative"
          title={`${drafterCount} detected ${pluralize(drafterCount, "drafter")}`}
          description="Assistant and MTP GGUF files are kept out of the primary-model list and offered on a profile's Speculative tab."
          icon={BotIcon}
          bodyClassName="p-0"
        >
          <ul className="divide-y divide-border">
            {drafters.map((drafter) => (
              <DrafterRow key={drafter.id} drafter={drafter} />
            ))}
          </ul>
        </Section>
      ) : null}

      {catalog.data && projectorCount > 0 ? (
        <Section
          label="Multimodal"
          title={`${projectorCount} detected ${pluralize(projectorCount, "projector")}`}
          description="Projectors are listed separately and attached automatically only when the match is unique."
          icon={ImageIcon}
          bodyClassName="p-0"
        >
          <ul className="divide-y divide-border">
            {catalog.data.projectors.map((projector) => (
              <li
                key={projector.id}
                className="flex items-center justify-between gap-4 px-4 py-3"
              >
                <div className="flex min-w-0 flex-col gap-1">
                  <span className="flex flex-wrap items-center gap-2">
                    <span className="truncate text-sm font-medium">
                      {projector.displayName}
                    </span>
                    <Badge variant="outline">
                      {projector.metadata.projectorType ?? "mmproj"}
                    </Badge>
                  </span>
                  <code className="truncate font-mono text-xs text-muted-foreground">
                    {projector.path}
                  </code>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <span className="text-xs text-muted-foreground">
                    {formatBytes(projector.sizeBytes)}
                  </span>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => void ipc.revealPath(projector.path)}
                  >
                    <FolderOpenIcon />
                    <span className="sr-only">Show {projector.displayName}</span>
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        </Section>
      ) : null}
    </div>
  );
}

function DrafterRow({ drafter }: { drafter: ModelRecord }) {
  return (
    <li className="flex items-center justify-between gap-4 px-4 py-3">
      <div className="flex min-w-0 flex-col gap-1">
        <span className="flex flex-wrap items-center gap-2">
          <span className="truncate text-sm font-medium">{drafter.displayName}</span>
          <Badge variant="secondary">Drafter</Badge>
          {drafter.metadata.architecture ? (
            <Badge variant="outline">{drafter.metadata.architecture}</Badge>
          ) : null}
          {drafter.metadata.sizeLabel ? (
            <Badge variant="outline">{drafter.metadata.sizeLabel}</Badge>
          ) : null}
        </span>
        <code className="truncate font-mono text-xs text-muted-foreground">
          {drafter.primaryPath ?? drafter.directory}
        </code>
        <span className="flex flex-wrap gap-x-3 text-xs text-muted-foreground">
          <span>{formatBytes(drafter.totalSizeBytes)}</span>
          {drafter.metadata.blockCount ? <span>{drafter.metadata.blockCount} blocks</span> : null}
          {drafter.metadata.tokenizerModel ? (
            <span>{drafter.metadata.tokenizerModel} tokenizer</span>
          ) : null}
        </span>
      </div>
      <Button
        variant="ghost"
        size="sm"
        disabled={!drafter.primaryPath}
        onClick={() => {
          if (drafter.primaryPath) void ipc.revealPath(drafter.primaryPath);
        }}
      >
        <FolderOpenIcon data-icon="inline-start" />
        Show file
      </Button>
    </li>
  );
}

type Selection =
  | { mode: "auto" }
  | { mode: "disabled" }
  | { mode: "custom"; path: string };

function ModelRow({
  model,
  projectors,
  pending,
  onSelection,
  onBrowse,
}: {
  model: ModelRecord;
  projectors: ProjectorRecord[];
  pending: boolean;
  onSelection: (selection: Selection) => void;
  onBrowse: () => Promise<void>;
}) {
  const selectedValue =
    model.projectorStatus === "disabled"
      ? DISABLED_VALUE
      : model.projectorStatus === "missingOverride"
        ? MISSING_VALUE
        : model.projectorStatus === "overridden" && model.projectorId
          ? model.projectorId
          : AUTOMATIC_VALUE;
  const selectedProjector = projectors.find(
    (projector) => projector.id === model.projectorId,
  );

  return (
    <li className="flex flex-col gap-3 px-4 py-4">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <span className="flex flex-wrap items-center gap-2">
            <span className="truncate text-sm font-semibold">{model.displayName}</span>
            {model.metadata.architecture ? (
              <Badge variant="secondary">{model.metadata.architecture}</Badge>
            ) : null}
            {model.metadata.sizeLabel ? (
              <Badge variant="outline">{model.metadata.sizeLabel}</Badge>
            ) : null}
            <Badge variant={model.complete ? "outline" : "destructive"}>
              <LayersIcon data-icon="inline-start" />
              {model.shards.length}/{model.expectedShards} {pluralize(model.expectedShards, "shard")}
            </Badge>
          </span>
          <code className="truncate font-mono text-xs text-muted-foreground">
            {model.primaryPath ?? model.directory}
          </code>
          <span className="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
            <span>{formatBytes(model.totalSizeBytes)}</span>
            <span>{model.metadata.tensorCount.toLocaleString()} tensors</span>
            {model.metadata.contextLength ? (
              <span>{model.metadata.contextLength.toLocaleString()} context</span>
            ) : null}
            {model.metadata.embeddingLength ? (
              <span>{model.metadata.embeddingLength.toLocaleString()} embedding</span>
            ) : null}
            {model.metadata.blockCount ? <span>{model.metadata.blockCount} blocks</span> : null}
            {model.metadata.tokenizerModel ? (
              <span>{model.metadata.tokenizerModel} tokenizer</span>
            ) : null}
            <span>GGUF v{model.metadata.version}</span>
          </span>
        </div>

        <Button
          variant="ghost"
          size="sm"
          disabled={!model.primaryPath}
          onClick={() => {
            if (model.primaryPath) {
              void ipc.revealPath(model.primaryPath);
            }
          }}
        >
          <FolderOpenIcon data-icon="inline-start" />
          Show file
        </Button>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border bg-muted/25 px-3 py-2.5">
        <div className="flex min-w-0 flex-col">
          <span className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
            Multimodal projector
          </span>
          <span className="truncate text-xs text-muted-foreground">
            {projectorDescription(model, selectedProjector)}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Select
            value={selectedValue}
            disabled={pending}
            onValueChange={(value) => {
              if (value === AUTOMATIC_VALUE) {
                onSelection({ mode: "auto" });
              } else if (value === DISABLED_VALUE) {
                onSelection({ mode: "disabled" });
              } else if (value !== MISSING_VALUE) {
                const projector = projectors.find((item) => item.id === value);
                if (projector) {
                  onSelection({ mode: "custom", path: projector.path });
                }
              }
            }}
          >
            <SelectTrigger className="w-56">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {model.projectorStatus === "missingOverride" ? (
                <SelectItem value={MISSING_VALUE} disabled>
                  Saved file missing
                </SelectItem>
              ) : null}
              <SelectItem value={AUTOMATIC_VALUE}>Automatic</SelectItem>
              <SelectItem value={DISABLED_VALUE}>No projector</SelectItem>
              {projectors.map((projector) => (
                <SelectItem key={projector.id} value={projector.id}>
                  {projector.displayName}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            variant="outline"
            size="sm"
            disabled={pending}
            onClick={() => void onBrowse()}
          >
            Browse…
          </Button>
        </div>
      </div>
    </li>
  );
}

function projectorDescription(
  model: ModelRecord,
  projector: ProjectorRecord | undefined,
): string {
  switch (model.projectorStatus) {
    case "auto":
      return `${projector?.displayName ?? "Projector"} matched automatically and unambiguously.`;
    case "overridden":
      return `${projector?.displayName ?? "Projector"} selected manually.`;
    case "ambiguous":
      return `${model.projectorCandidates.length} compatible projectors found; choose one manually.`;
    case "disabled":
      return "Automatic projector selection is disabled for this model.";
    case "missingOverride":
      return "The manually selected projector is missing or no longer valid.";
    case "none":
      return "No unambiguous compatible projector found.";
  }
}
