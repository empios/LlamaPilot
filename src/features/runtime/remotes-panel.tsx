import { DownloadIcon, PlusIcon, TrashIcon } from "lucide-react";
import { useState } from "react";

import { ErrorPanel } from "@/components/error-panel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  useAddSourceRemote,
  useRemoveSourceRemote,
  useSourceRemotes,
} from "@/hooks/use-sources";

interface RemotesPanelProps {
  sourceId: string;
  onFetchRemote: (remote: string) => void;
  fetchPending: boolean;
}

export function RemotesPanel({ sourceId, onFetchRemote, fetchPending }: RemotesPanelProps) {
  const remotes = useSourceRemotes(sourceId);
  const addRemote = useAddSourceRemote();
  const removeRemote = useRemoveSourceRemote();

  const [name, setName] = useState("");
  const [url, setUrl] = useState("");

  const canAdd = name.trim().length > 0 && url.trim().length > 0;

  return (
    <div className="flex flex-col">
      {remotes.isPending ? <Skeleton className="m-4 h-16" /> : null}
      {remotes.isError ? <ErrorPanel error={remotes.error} className="m-4" /> : null}

      {remotes.data ? (
        <ul className="divide-y divide-border">
          {remotes.data.map((remote) => {
            const isOrigin = remote.name === "origin";

            return (
              <li
                key={remote.name}
                className="flex items-center justify-between gap-4 px-4 py-2.5"
              >
                <div className="flex min-w-0 flex-col gap-0.5">
                  <span className="flex items-center gap-2">
                    <span className="text-sm font-medium">{remote.name}</span>
                    {isOrigin ? (
                      <Badge variant="secondary" className="font-normal">
                        primary
                      </Badge>
                    ) : null}
                  </span>
                  <code className="truncate font-mono text-xs text-muted-foreground">
                    {remote.fetchUrl ?? "no fetch URL"}
                  </code>
                </div>

                <div className="flex shrink-0 items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={fetchPending}
                    onClick={() => onFetchRemote(remote.name)}
                  >
                    <DownloadIcon data-icon="inline-start" />
                    Fetch
                  </Button>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <span>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          disabled={isOrigin || removeRemote.isPending}
                          onClick={() =>
                            removeRemote.mutate({ id: sourceId, name: remote.name })
                          }
                        >
                          <TrashIcon />
                          <span className="sr-only">Remove remote {remote.name}</span>
                        </Button>
                      </span>
                    </TooltipTrigger>
                    <TooltipContent>
                      {isOrigin
                        ? "The primary remote cannot be removed"
                        : `Remove ${remote.name}`}
                    </TooltipContent>
                  </Tooltip>
                </div>
              </li>
            );
          })}
        </ul>
      ) : null}

      <div className="flex flex-wrap items-center gap-2 border-t border-border bg-muted/30 px-4 py-3">
        <Input
          value={name}
          placeholder="Remote name"
          aria-label="Remote name"
          className="w-40"
          onChange={(event) => setName(event.currentTarget.value)}
        />
        <Input
          value={url}
          placeholder="https://github.com/some-user/llama.cpp"
          aria-label="Remote repository URL"
          className="min-w-56 flex-1"
          onChange={(event) => setUrl(event.currentTarget.value)}
        />
        <Button
          variant="outline"
          disabled={!canAdd || addRemote.isPending}
          onClick={() =>
            addRemote.mutate(
              { id: sourceId, name: name.trim(), url: url.trim() },
              {
                onSuccess: () => {
                  setName("");
                  setUrl("");
                },
              },
            )
          }
        >
          <PlusIcon data-icon="inline-start" />
          Add remote
        </Button>
      </div>
    </div>
  );
}
