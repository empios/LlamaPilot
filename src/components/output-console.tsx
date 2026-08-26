import { useEffect, useRef } from "react";

import { cn } from "@/lib/utils";
import type { ProgressEvent } from "@/types/sources";

export interface ConsoleLine {
  stream: "stdout" | "stderr";
  text: string;
}

interface OutputConsoleProps {
  lines: ConsoleLine[];
  emptyMessage?: string;
  className?: string;
}

/**
 * Auto-scrolling raw output view.
 *
 * stderr is not styled as an error: Git and CMake write ordinary progress to stderr, so colouring
 * it red would make every successful operation look broken.
 */
export function OutputConsole({
  lines,
  emptyMessage = "No output yet.",
  className,
}: OutputConsoleProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: "end" });
  }, [lines.length]);

  return (
    <div
      className={cn(
        "h-56 overflow-auto rounded-md border border-border bg-muted/40 p-3 font-mono text-xs",
        className,
      )}
    >
      {lines.length === 0 ? (
        <p className="text-muted-foreground">{emptyMessage}</p>
      ) : (
        lines.map((line, index) => (
          <div
            key={`${index}-${line.text}`}
            className="break-all whitespace-pre-wrap text-foreground/90"
          >
            {line.text}
          </div>
        ))
      )}
      <div ref={bottomRef} />
    </div>
  );
}

/** Reduces a stream of progress events into console lines. */
export function appendProgressEvent(
  lines: ConsoleLine[],
  event: ProgressEvent,
): ConsoleLine[] {
  switch (event.kind) {
    case "started":
      return [{ stream: "stdout", text: `> ${event.label}` }];
    case "output":
      return [...lines, { stream: event.stream, text: event.text }];
    case "finished":
      return [
        ...lines,
        {
          stream: event.success ? "stdout" : "stderr",
          text: event.success ? "> Done" : "> Failed",
        },
      ];
    default: {
      const exhaustive: never = event;
      return exhaustive;
    }
  }
}
