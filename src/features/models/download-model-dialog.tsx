import { DownloadIcon, SearchIcon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

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
import { Progress } from "@/components/ui/progress";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import {
  useDownloadHuggingFaceModel,
  useInspectHuggingFaceRepository,
} from "@/hooks/use-models";
import { formatBytes, pluralize } from "@/lib/format";
import { createModelDownloadChannel, ipc } from "@/lib/ipc";
import type { HuggingFaceRepository } from "@/types/models";

interface DownloadModelDialogProps {
  open: boolean;
  directories: string[];
  onOpenChange: (open: boolean) => void;
}

interface ByteProgress {
  downloaded: number;
  total: number;
}

export function DownloadModelDialog({
  open,
  directories,
  onOpenChange,
}: DownloadModelDialogProps) {
  const inspectRepository = useInspectHuggingFaceRepository();
  const downloadModel = useDownloadHuggingFaceModel();
  const [repositoryInput, setRepositoryInput] = useState("");
  const [repository, setRepository] = useState<HuggingFaceRepository | null>(null);
  const [selectionId, setSelectionId] = useState("");
  const [destination, setDestination] = useState(directories[0] ?? "");
  const [currentFile, setCurrentFile] = useState<string | null>(null);
  const [progress, setProgress] = useState<ByteProgress>({ downloaded: 0, total: 0 });
  const [cancelling, setCancelling] = useState(false);

  const selected = useMemo(
    () => repository?.selections.find((selection) => selection.id === selectionId),
    [repository, selectionId],
  );
  const percentage =
    progress.total > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
      : 0;

  useEffect(() => {
    if (!directories.includes(destination)) {
      setDestination(directories[0] ?? "");
    }
  }, [destination, directories]);

  const loadRepository = () => {
    setRepository(null);
    setSelectionId("");
    setCurrentFile(null);
    inspectRepository.mutate(repositoryInput.trim(), {
      onSuccess: (result) => {
        setRepository(result);
        setSelectionId(
          result.selections.find((selection) => selection.complete)?.id ?? "",
        );
      },
    });
  };

  const startDownload = () => {
    if (!repository || !selected || !destination) return;
    setCancelling(false);
    setCurrentFile(null);
    setProgress({ downloaded: 0, total: selected.totalSizeBytes });
    const channel = createModelDownloadChannel((nextEvent) => {
      if (nextEvent.type === "fileStarted") {
        setCurrentFile(nextEvent.path);
      }
      if (nextEvent.type === "started") {
        setProgress({ downloaded: 0, total: nextEvent.totalBytes });
      } else if (nextEvent.type === "progress") {
        setProgress({
          downloaded: nextEvent.downloadedBytes,
          total: nextEvent.totalBytes,
        });
      } else if (nextEvent.type === "finished") {
        setProgress({
          downloaded: nextEvent.totalBytes,
          total: nextEvent.totalBytes,
        });
      }
    });
    downloadModel.mutate(
      {
        request: {
          repositoryId: repository.repositoryId,
          revision: repository.revision,
          selectionId: selected.id,
          destinationDirectory: destination,
        },
        channel,
      },
      {
        onSuccess: () => {
          setRepositoryInput("");
          setRepository(null);
          setSelectionId("");
          setCurrentFile(null);
          setProgress({ downloaded: 0, total: 0 });
          onOpenChange(false);
        },
        onSettled: () => setCancelling(false),
      },
    );
  };

  const downloading = downloadModel.isPending;

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen && downloading) return;
        onOpenChange(nextOpen);
      }}
    >
      <DialogContent className="sm:max-w-2xl" showCloseButton={!downloading}>
        <DialogHeader>
          <DialogTitle>Download a GGUF model</DialogTitle>
          <DialogDescription>
            Choose a public Hugging Face repository and the exact file you want. LlamaPilot
            does not recommend or substitute a model.
          </DialogDescription>
        </DialogHeader>

        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="hugging-face-repository">Repository</FieldLabel>
            <div className="flex gap-2">
              <Input
                id="hugging-face-repository"
                value={repositoryInput}
                placeholder="owner/model-GGUF or its Hugging Face URL"
                disabled={downloading}
                onChange={(inputEvent) => setRepositoryInput(inputEvent.currentTarget.value)}
                onKeyDown={(keyEvent) => {
                  if (keyEvent.key === "Enter" && repositoryInput.trim()) {
                    loadRepository();
                  }
                }}
              />
              <Button
                variant="outline"
                disabled={
                  !repositoryInput.trim() || inspectRepository.isPending || downloading
                }
                onClick={loadRepository}
              >
                {inspectRepository.isPending ? (
                  <Spinner data-icon="inline-start" />
                ) : (
                  <SearchIcon data-icon="inline-start" />
                )}
                {inspectRepository.isPending ? "Loading…" : "Load files"}
              </Button>
            </div>
            <FieldDescription>
              Public, ungated repositories are supported in this version.
            </FieldDescription>
          </Field>

          {repository ? (
            repository.selections.length > 0 ? (
              <>
                <Field>
                  <FieldLabel>GGUF file</FieldLabel>
                  <Select
                    value={selectionId}
                    disabled={downloading}
                    onValueChange={setSelectionId}
                  >
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Choose a GGUF file" />
                    </SelectTrigger>
                    <SelectContent>
                      {repository.selections.map((selection) => (
                        <SelectItem
                          key={selection.id}
                          value={selection.id}
                          disabled={!selection.complete}
                        >
                          {selection.displayName} · {formatBytes(selection.totalSizeBytes)}
                          {selection.expectedFiles > 1
                            ? ` · ${selection.files.length}/${selection.expectedFiles} shards`
                            : ""}
                          {!selection.complete ? " · incomplete" : ""}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {selected ? (
                    <FieldDescription>
                      {selected.files.length} {pluralize(selected.files.length, "file")} · pinned
                      to revision {repository.revision.slice(0, 8)}
                    </FieldDescription>
                  ) : null}
                </Field>

                <Field>
                  <FieldLabel>Model folder</FieldLabel>
                  <Select
                    value={destination}
                    disabled={downloading}
                    onValueChange={setDestination}
                  >
                    <SelectTrigger className="w-full font-mono text-xs">
                      <SelectValue placeholder="Choose a configured model folder" />
                    </SelectTrigger>
                    <SelectContent>
                      {directories.map((directory) => (
                        <SelectItem key={directory} value={directory}>
                          {directory}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <FieldDescription>
                    Files are saved directly here, then the model catalog is refreshed
                    automatically.
                  </FieldDescription>
                </Field>
              </>
            ) : (
              <div className="rounded-lg border border-border bg-muted/30 p-4 text-sm text-muted-foreground">
                This repository contains no GGUF files.
              </div>
            )
          ) : null}

          {downloading ? (
            <div className="flex flex-col gap-2 rounded-lg border border-border bg-muted/30 p-4">
              <div className="flex items-center justify-between gap-3 text-sm">
                <span className="min-w-0 truncate font-medium">
                  {cancelling ? "Cancelling…" : currentFile ?? "Downloading model…"}
                </span>
                <span className="shrink-0 text-muted-foreground">{percentage}%</span>
              </div>
              <Progress value={percentage} aria-label="Model download progress" />
              <span className="text-xs text-muted-foreground">
                {formatBytes(progress.downloaded)} / {formatBytes(progress.total)}
              </span>
            </div>
          ) : null}
        </FieldGroup>

        <DialogFooter>
          {downloading ? (
            <Button
              variant="destructive"
              disabled={cancelling}
              onClick={() => {
                setCancelling(true);
                void ipc.cancelModelDownload();
              }}
            >
              {cancelling ? <Spinner data-icon="inline-start" /> : null}
              {cancelling ? "Cancelling…" : "Cancel download"}
            </Button>
          ) : (
            <Button
              disabled={!repository || !selected?.complete || !destination}
              onClick={startDownload}
            >
              <DownloadIcon data-icon="inline-start" />
              Download to model folder
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
