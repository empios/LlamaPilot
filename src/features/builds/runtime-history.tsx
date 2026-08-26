import {
  FolderOpenIcon,
  LayersIcon,
  SlidersHorizontalIcon,
  TrashIcon,
} from "lucide-react";
import { useState } from "react";

import { ErrorPanel } from "@/components/error-panel";
import { Section } from "@/components/section";
import { RuntimeCapabilitiesDialog } from "@/features/builds/runtime-capabilities-dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useDeleteRuntime, useRuntimes } from "@/hooks/use-build";
import { formatBytes, formatTimestamp, pluralize } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import {
  backendLabel,
  configurationLabel,
  type RuntimeRecord,
} from "@/types/build";

export function RuntimeHistory() {
  const runtimes = useRuntimes();
  const deleteRuntime = useDeleteRuntime();
  const [pendingDelete, setPendingDelete] = useState<RuntimeRecord | null>(null);
  const [selectedRuntimeId, setSelectedRuntimeId] = useState<string | null>(null);

  const count = runtimes.data?.length ?? 0;
  const selectedRuntime =
    runtimes.data?.find((runtime) => runtime.id === selectedRuntimeId) ?? null;

  return (
    <Section
      label="Runtimes"
      title="Successful build history"
      description="Every build is a separate self-describing snapshot, including repeated builds of the same commit."
      icon={LayersIcon}
      bodyClassName={count > 0 ? "p-0" : "p-4"}
    >
      {runtimes.isPending ? <Skeleton className="h-20 w-full" /> : null}
      {runtimes.isError ? <ErrorPanel error={runtimes.error} /> : null}

      {runtimes.data && count === 0 ? (
        <p className="text-sm text-muted-foreground">
          No runtimes yet. A successful build creates the first one.
        </p>
      ) : null}

      {runtimes.data && count > 0 ? (
        <ul className="divide-y divide-border">
          {runtimes.data.map((runtime) => (
            <li
              key={runtime.id}
              className="flex items-center justify-between gap-4 px-4 py-3"
            >
              <div className="flex min-w-0 flex-col gap-1">
                <span className="flex flex-wrap items-center gap-2">
                  <Badge variant="secondary">{backendLabel(runtime.backend)}</Badge>
                  <span className="truncate text-sm font-medium">{runtime.branch}</span>
                  <code className="font-mono text-xs text-muted-foreground">
                    {runtime.shortCommit}
                  </code>
                  <Badge variant="outline" className="font-mono font-normal">
                    {runtime.capabilities?.version ?? "Not inspected"}
                  </Badge>
                </span>
                <span className="truncate text-xs text-muted-foreground">
                  {formatTimestamp(runtime.buildDate)} ·{" "}
                  {configurationLabel(runtime.configuration)} · {runtime.generator} ·{" "}
                  {runtime.fileCount} {pluralize(runtime.fileCount, "file")} ·{" "}
                  {formatBytes(runtime.sizeBytes)}
                </span>
              </div>

              <div className="flex shrink-0 items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setSelectedRuntimeId(runtime.id)}
                >
                  <SlidersHorizontalIcon data-icon="inline-start" />
                  {runtime.capabilities ? "Capabilities" : "Inspect"}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => void ipc.revealPath(runtime.directory)}
                >
                  <FolderOpenIcon data-icon="inline-start" />
                  Open
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  disabled={deleteRuntime.isPending}
                  onClick={() => setPendingDelete(runtime)}
                >
                  <TrashIcon />
                  <span className="sr-only">Delete runtime {runtime.shortCommit}</span>
                </Button>
              </div>
            </li>
          ))}
        </ul>
      ) : null}

      <AlertDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingDelete(null);
          }
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete this runtime permanently?</AlertDialogTitle>
            <AlertDialogDescription>
              {pendingDelete
                ? `${pendingDelete.branch} @ ${pendingDelete.shortCommit} and all copied binaries will be removed from ${pendingDelete.directory}.`
                : "The selected runtime and its copied binaries will be removed."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              disabled={deleteRuntime.isPending}
              onClick={() => {
                if (pendingDelete) {
                  deleteRuntime.mutate(pendingDelete.id, {
                    onSuccess: () => setPendingDelete(null),
                  });
                }
              }}
            >
              Delete runtime
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <RuntimeCapabilitiesDialog
        runtime={selectedRuntime}
        open={selectedRuntime !== null}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedRuntimeId(null);
          }
        }}
      />
    </Section>
  );
}
