import {
  CloudDownloadIcon,
  DownloadIcon,
  FolderOpenIcon,
  GitBranchIcon,
  HammerIcon,
  MoreVerticalIcon,
  RefreshCwIcon,
  TerminalIcon,
  TrashIcon,
} from "lucide-react";
import { useState } from "react";

import { ErrorPanel } from "@/components/error-panel";
import { OutputConsole, appendProgressEvent, type ConsoleLine } from "@/components/output-console";
import { Section } from "@/components/section";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { RemoveSourceDialog } from "@/features/runtime/remove-source-dialog";
import { RemotesPanel } from "@/features/runtime/remotes-panel";
import { SourceStatusSummary } from "@/features/runtime/source-status-summary";
import { SwitchRefPanel } from "@/features/runtime/switch-ref-panel";
import { useFetchSource, useSourceStatus, useUpdateSource } from "@/hooks/use-sources";
import { createProgressChannel, ipc } from "@/lib/ipc";
import { useNavigationStore } from "@/stores/navigation-store";

export function SourceDetail({ sourceId }: { sourceId: string }) {
  const status = useSourceStatus(sourceId);
  const fetchSource = useFetchSource();
  const updateSource = useUpdateSource();
  const navigate = useNavigationStore((state) => state.navigate);

  const [lines, setLines] = useState<ConsoleLine[]>([]);
  const [removeOpen, setRemoveOpen] = useState(false);

  const runFetch = (remote: string | null) => {
    setLines([]);
    const channel = createProgressChannel((event) => {
      setLines((current) => appendProgressEvent(current, event));
    });
    fetchSource.mutate({ id: sourceId, remote, channel });
  };

  if (status.isPending) {
    return <Skeleton className="h-72 w-full" />;
  }

  if (status.isError) {
    return <ErrorPanel error={status.error} />;
  }

  const { source, directoryExists, git } = status.data;
  const hasTrackedChanges = git.staged + git.unstaged + git.conflicted > 0;
  const canUpdate =
    directoryExists && !git.detached && !hasTrackedChanges && Boolean(git.upstream);

  const switchBlockedReason = hasTrackedChanges
    ? "The working tree has uncommitted changes to tracked files. Git would refuse to switch, so commit, stash, or discard them first."
    : undefined;

  return (
    <div className="flex min-w-0 flex-col gap-6">
      <Section
        label="Repository"
        title={source.name}
        description={source.repository}
        icon={GitBranchIcon}
        actions={
          <>
            <Button
              variant="outline"
              size="sm"
              disabled={!directoryExists || fetchSource.isPending}
              onClick={() => runFetch(null)}
            >
              {fetchSource.isPending ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <RefreshCwIcon data-icon="inline-start" />
              )}
              Fetch
            </Button>
            <Button
              size="sm"
              disabled={!canUpdate || updateSource.isPending}
              onClick={() => updateSource.mutate(sourceId)}
            >
              {updateSource.isPending ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <DownloadIcon data-icon="inline-start" />
              )}
              Update
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon-sm">
                  <MoreVerticalIcon />
                  <span className="sr-only">More actions for {source.name}</span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuGroup>
                  <DropdownMenuItem onSelect={() => void ipc.revealPath(source.directory)}>
                    <FolderOpenIcon />
                    Open folder
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => navigate("build")}>
                    <HammerIcon />
                    Build llama.cpp
                  </DropdownMenuItem>
                  <DropdownMenuItem variant="destructive" onSelect={() => setRemoveOpen(true)}>
                    <TrashIcon />
                    Remove source
                  </DropdownMenuItem>
                </DropdownMenuGroup>
              </DropdownMenuContent>
            </DropdownMenu>
          </>
        }
        bodyClassName="flex flex-col gap-5 p-4"
      >
        {directoryExists ? (
          <SourceStatusSummary status={status.data} />
        ) : (
          <ErrorPanel
            error={{
              code: "invalidPath",
              message: "The source folder no longer exists on disk.",
              hint: "Restore the folder, or remove this source and add it again.",
              details: source.directory,
            }}
          />
        )}
      </Section>

      {lines.length > 0 ? (
        <Section
          label="Git output"
          title="Raw progress from the last operation"
          icon={TerminalIcon}
          bodyClassName="p-4"
          actions={
            <Button variant="ghost" size="sm" onClick={() => setLines([])}>
              Clear
            </Button>
          }
        >
          <OutputConsole lines={lines} />
        </Section>
      ) : null}

      {directoryExists ? (
        <>
          <Section
            label="Version"
            title="Switch ref"
            icon={GitBranchIcon}
            bodyClassName="p-4"
          >
            <SwitchRefPanel sourceId={sourceId} blockedReason={switchBlockedReason} />
          </Section>

          <Section
            label="Remotes"
            title="Where refs are fetched from"
            description="Add a fork to use its experimental branches without changing this app."
            icon={CloudDownloadIcon}
            bodyClassName="p-0"
          >
            <RemotesPanel
              sourceId={sourceId}
              fetchPending={fetchSource.isPending}
              onFetchRemote={(remote) => runFetch(remote)}
            />
          </Section>
        </>
      ) : null}

      <RemoveSourceDialog
        sourceId={sourceId}
        sourceName={source.name}
        directory={source.directory}
        open={removeOpen}
        onOpenChange={setRemoveOpen}
      />
    </div>
  );
}
