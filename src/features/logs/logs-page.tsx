import {
  ArrowDownIcon,
  CopyIcon,
  FolderOpenIcon,
  PauseIcon,
  PlayIcon,
  SearchIcon,
  TerminalIcon,
  TrashIcon,
  XIcon,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import { ErrorPanel } from "@/components/error-panel";
import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useClearServerLogs, useServerLogs, useServerStatus } from "@/hooks/use-server";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import {
  serverIsActive,
  serverStateLabel,
  type ServerLifecycleState,
  type ServerLogEntry,
} from "@/types/server";

type LevelFilter = "all" | ServerLogEntry["level"];
type StreamFilter = "all" | ServerLogEntry["stream"];

export function LogsPage() {
  const logs = useServerLogs();
  const server = useServerStatus();
  const clearLogs = useClearServerLogs();
  const [search, setSearch] = useState("");
  const [level, setLevel] = useState<LevelFilter>("all");
  const [stream, setStream] = useState<StreamFilter>("all");
  const [paused, setPaused] = useState(false);
  const [pausedEntries, setPausedEntries] = useState<ServerLogEntry[]>([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const [rawMode, setRawMode] = useState(false);
  const consoleRef = useRef<HTMLDivElement>(null);

  const liveEntries = logs.data?.entries ?? [];
  const sourceEntries = paused ? pausedEntries : liveEntries;
  const filtered = useMemo(() => {
    const needle = search.trim().toLocaleLowerCase();
    return sourceEntries.filter((entry) => {
      if (level !== "all" && entry.level !== level) return false;
      if (stream !== "all" && entry.stream !== stream) return false;
      return !needle || entry.text.toLocaleLowerCase().includes(needle);
    });
  }, [level, search, sourceEntries, stream]);

  const filtersActive = search.trim() !== "" || level !== "all" || stream !== "all";
  const serverState = server.data?.state ?? "stopped";
  const lastSequence = filtered.at(-1)?.sequence;

  useEffect(() => {
    if (autoScroll && !paused && consoleRef.current) {
      consoleRef.current.scrollTop = consoleRef.current.scrollHeight;
    }
  }, [autoScroll, lastSequence, paused]);

  const togglePause = () => {
    if (!paused) setPausedEntries(liveEntries);
    setPaused((value) => !value);
  };

  const resetFilters = () => {
    setSearch("");
    setLevel("all");
    setStream("all");
  };

  const copyVisible = async () => {
    await navigator.clipboard.writeText(filtered.map((entry) => entry.text).join("\n"));
    toast.success(`${filtered.length} visible log lines copied`);
  };

  const clearVisibleLogs = () => {
    setPausedEntries([]);
    clearLogs.mutate();
  };

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        eyebrow="System"
        title="Server logs"
        description="Inspect live llama-server output without leaving the control panel."
        actions={
          logs.data?.currentFile ? (
            <Button
              variant="outline"
              onClick={() => void ipc.revealPath(logs.data!.currentFile!)}
            >
              <FolderOpenIcon data-icon="inline-start" />
              Open log file
            </Button>
          ) : null
        }
      />

      {logs.error ? <ErrorPanel error={logs.error} /> : null}
      {server.error ? <ErrorPanel error={server.error} /> : null}

      <section className="overflow-hidden rounded-xl border border-border bg-card shadow-sm">
        <header className="flex flex-wrap items-center justify-between gap-4 border-b border-border px-4 py-3.5">
          <div className="flex min-w-0 items-center gap-3">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border bg-muted/50 text-muted-foreground">
              <TerminalIcon className="size-4" />
            </span>
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <h2 className="truncate text-sm font-semibold">
                  {server.data?.profileName ?? "llama-server output"}
                </h2>
                <span
                  className={cn(
                    "size-2 shrink-0 rounded-full",
                    serverStateDotClass(serverState),
                  )}
                />
              </div>
              <p className="truncate text-xs text-muted-foreground">
                {server.data?.host && server.data.port
                  ? `${server.data.host}:${server.data.port}`
                  : "Start a profile to begin capturing output"}
              </p>
            </div>
          </div>

          <div className="flex items-center divide-x divide-border text-xs">
            <span className="pr-3 font-medium">{serverStateLabel(serverState)}</span>
            <span className="px-3 text-muted-foreground">
              <strong className="font-medium text-foreground">{liveEntries.length}</strong> lines
            </span>
            {logs.data?.droppedEntries ? (
              <span className="pl-3 text-warning">
                {logs.data.droppedEntries} dropped
              </span>
            ) : null}
          </div>
        </header>

        <div className="border-b border-border bg-muted/20 p-3">
          <div className="grid min-w-0 grid-cols-1 gap-2 sm:grid-cols-[minmax(220px,1fr)_144px_144px]">
            <div className="relative min-w-56 flex-1">
              <SearchIcon className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Search server output"
                className="pr-8 pl-8"
              />
              {search ? (
                <button
                  type="button"
                  aria-label="Clear search"
                  className="absolute top-1/2 right-2 flex size-5 -translate-y-1/2 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                  onClick={() => setSearch("")}
                >
                  <XIcon className="size-3.5" />
                </button>
              ) : null}
            </div>

            <Select value={level} onValueChange={(value) => setLevel(value as LevelFilter)}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All levels</SelectItem>
                <SelectItem value="error">Errors</SelectItem>
                <SelectItem value="warn">Warnings</SelectItem>
                <SelectItem value="info">Info</SelectItem>
                <SelectItem value="debug">Debug</SelectItem>
                <SelectItem value="trace">Trace</SelectItem>
              </SelectContent>
            </Select>

            <Select value={stream} onValueChange={(value) => setStream(value as StreamFilter)}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All streams</SelectItem>
                <SelectItem value="system">System</SelectItem>
                <SelectItem value="stdout">stdout</SelectItem>
                <SelectItem value="stderr">stderr</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="mt-2 flex flex-wrap items-center justify-between gap-2 border-t border-border/70 pt-2">
            <div className="flex flex-wrap items-center gap-1">
              <Button variant={paused ? "secondary" : "ghost"} size="sm" onClick={togglePause}>
                {paused ? (
                  <PlayIcon data-icon="inline-start" />
                ) : (
                  <PauseIcon data-icon="inline-start" />
                )}
                {paused ? "Resume" : "Pause"}
              </Button>
              <Button
                variant={autoScroll ? "secondary" : "ghost"}
                size="sm"
                onClick={() => setAutoScroll((value) => !value)}
              >
                <ArrowDownIcon data-icon="inline-start" />
                Follow
              </Button>
              <Button
                variant={rawMode ? "secondary" : "ghost"}
                size="sm"
                onClick={() => setRawMode((value) => !value)}
              >
                Raw
              </Button>
            </div>

            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                disabled={filtered.length === 0}
                onClick={() => void copyVisible()}
              >
                <CopyIcon data-icon="inline-start" />
                Copy
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="text-muted-foreground hover:text-destructive"
                disabled={liveEntries.length === 0 || clearLogs.isPending}
                onClick={clearVisibleLogs}
              >
                <TrashIcon data-icon="inline-start" />
                Clear
              </Button>
            </div>
          </div>
        </div>

        {paused ? (
          <div className="flex items-center justify-between gap-3 border-b border-warning/30 bg-warning/10 px-4 py-2 text-xs text-warning">
            <span>Output is paused at {pausedEntries.length} lines. New entries are still being captured.</span>
            <button type="button" className="font-semibold hover:underline" onClick={togglePause}>
              Resume live view
            </button>
          </div>
        ) : null}

        <div
          ref={consoleRef}
          onScroll={() => {
            const consoleElement = consoleRef.current;
            if (!consoleElement || paused) return;
            const distanceFromBottom =
              consoleElement.scrollHeight - consoleElement.scrollTop - consoleElement.clientHeight;
            setAutoScroll(distanceFromBottom < 32);
          }}
          className="h-[480px] overflow-auto bg-[#0b0d10] text-xs text-zinc-200"
        >
          {filtered.length === 0 ? (
            <LogEmptyState
              hasLogs={liveEntries.length > 0}
              serverActive={serverIsActive(serverState)}
              filtersActive={filtersActive}
              onResetFilters={resetFilters}
            />
          ) : rawMode ? (
            <div className="py-2 font-mono">
              {filtered.map((entry, index) => (
                <div
                  key={entry.sequence}
                  className="group flex min-w-0 gap-3 px-4 py-1 leading-5 hover:bg-white/[0.035]"
                >
                  <span className="w-10 shrink-0 select-none text-right text-zinc-700">
                    {index + 1}
                  </span>
                  <span className="min-w-0 break-words whitespace-pre-wrap text-zinc-300">
                    {entry.text}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <div className="min-w-[680px] py-2 font-mono">
              {filtered.map((entry) => (
                <LogRow key={entry.sequence} entry={entry} />
              ))}
            </div>
          )}
        </div>

        <footer className="flex flex-wrap items-center justify-between gap-2 border-t border-border bg-muted/20 px-4 py-2 text-[11px] text-muted-foreground">
          <span>
            Showing <strong className="font-medium text-foreground">{filtered.length}</strong> of{" "}
            {sourceEntries.length} lines
          </span>
          <div className="flex items-center gap-3">
            {filtersActive ? (
              <button type="button" className="hover:text-foreground" onClick={resetFilters}>
                Reset filters
              </button>
            ) : null}
            <span className="flex items-center gap-1.5">
              <span
                className={cn(
                  "size-1.5 rounded-full",
                  autoScroll && !paused ? "bg-emerald-400" : "bg-muted-foreground/50",
                )}
              />
              {paused ? "Snapshot" : autoScroll ? "Following output" : "Scroll unlocked"}
            </span>
          </div>
        </footer>
      </section>
    </div>
  );
}

function LogRow({ entry }: { entry: ServerLogEntry }) {
  return (
    <div className="grid grid-cols-[88px_74px_minmax(0,1fr)] items-start gap-x-3 px-4 py-1.5 leading-5 hover:bg-white/[0.035]">
      <time className="select-none text-zinc-600">{formatLogTime(entry.timestamp)}</time>
      <span
        className={cn(
          "w-fit rounded px-1.5 py-px text-[10px] leading-[18px] font-semibold tracking-wide uppercase",
          levelClass(entry.level),
        )}
      >
        {entry.level}
      </span>
      <div className="flex min-w-0 items-start gap-2">
        <span className="mt-px shrink-0 text-[10px] leading-[18px] text-zinc-600">
          {entry.stream}
        </span>
        <span className="min-w-0 break-words whitespace-pre-wrap text-zinc-300">
          {entry.fact ? (
            <span className="mr-2 inline-flex rounded border border-indigo-400/20 bg-indigo-400/10 px-1.5 font-sans text-[10px] leading-[18px] font-medium text-indigo-300">
              {factLabel(entry.fact)}
            </span>
          ) : null}
          {entry.text}
        </span>
      </div>
    </div>
  );
}

function LogEmptyState({
  hasLogs,
  serverActive,
  filtersActive,
  onResetFilters,
}: {
  hasLogs: boolean;
  serverActive: boolean;
  filtersActive: boolean;
  onResetFilters: () => void;
}) {
  const title = hasLogs ? "No matching output" : serverActive ? "Waiting for output" : "Server is offline";
  const description = hasLogs
    ? "Try a different search term or reset the active filters."
    : serverActive
      ? "New server messages will appear here automatically."
      : "Start a launch profile to stream llama-server output here.";

  return (
    <div className="grid h-full min-h-96 place-items-center p-8">
      <div className="flex max-w-sm flex-col items-center text-center">
        <span className="mb-4 flex size-11 items-center justify-center rounded-xl border border-white/10 bg-white/[0.04] text-zinc-500">
          <TerminalIcon className="size-5" />
        </span>
        <h3 className="font-sans text-sm font-semibold text-zinc-200">{title}</h3>
        <p className="mt-1.5 font-sans text-xs leading-5 text-zinc-500">{description}</p>
        {filtersActive ? (
          <Button variant="outline" size="sm" className="mt-4" onClick={onResetFilters}>
            Reset filters
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function formatLogTime(timestamp: string): string {
  return new Date(timestamp).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    fractionalSecondDigits: 3,
  });
}

function factLabel(fact: NonNullable<ServerLogEntry["fact"]>): string {
  const labels = {
    modelLoad: "model",
    gpuOffload: "GPU",
    kvCache: "KV cache",
    listening: "network",
    throughput: "speed",
  } as const;
  return labels[fact];
}

function levelClass(level: ServerLogEntry["level"]): string {
  const classes = {
    trace: "bg-zinc-800 text-zinc-500",
    debug: "bg-zinc-800 text-zinc-400",
    info: "bg-sky-400/10 text-sky-300",
    warn: "bg-amber-400/10 text-amber-300",
    error: "bg-red-400/10 text-red-300",
  } as const;
  return classes[level];
}

function serverStateDotClass(state: ServerLifecycleState): string {
  if (state === "crashed") return "bg-destructive";
  if (state === "starting" || state === "loading" || state === "stopping") {
    return "animate-pulse bg-warning";
  }
  if (state === "ready" || state === "busy") return "bg-success";
  return "bg-muted-foreground/40";
}
