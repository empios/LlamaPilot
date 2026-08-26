import {
  CheckIcon,
  CircleDashedIcon,
  RefreshCwIcon,
  TriangleAlertIcon,
  WrenchIcon,
} from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";

import { ErrorPanel } from "@/components/error-panel";
import { Section } from "@/components/section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useToolchain } from "@/hooks/use-build";
import { queryKeys } from "@/lib/query-keys";
import { cn } from "@/lib/utils";
import type { ToolStatus } from "@/types/build";

export function ToolchainPanel() {
  const toolchain = useToolchain();
  const queryClient = useQueryClient();

  const missingRequired =
    toolchain.data?.tools.filter(
      (tool) => !tool.found && tool.requirement !== "optional",
    ) ?? [];

  return (
    <Section
      label="Toolchain"
      title="Build environment"
      description="Detected on this machine. CMake reports its own generators, so a new Visual Studio works without an update here."
      icon={WrenchIcon}
      bodyClassName="p-0"
      actions={
        <Button
          variant="outline"
          size="sm"
          disabled={toolchain.isFetching}
          onClick={() =>
            void queryClient.invalidateQueries({ queryKey: queryKeys.toolchain })
          }
        >
          <RefreshCwIcon data-icon="inline-start" />
          Re-detect
        </Button>
      }
    >
      {toolchain.isPending ? <Skeleton className="m-4 h-40" /> : null}
      {toolchain.isError ? <ErrorPanel error={toolchain.error} className="m-4" /> : null}

      {toolchain.data ? (
        <>
          <ul className="divide-y divide-border">
            {toolchain.data.tools.map((tool) => (
              <ToolRow key={tool.id} tool={tool} />
            ))}
          </ul>

          {missingRequired.length > 0 ? (
            <div className="border-t border-border bg-muted/30 px-4 py-3">
              <p className="text-xs text-muted-foreground">
                {missingRequired.length} required{" "}
                {missingRequired.length === 1 ? "tool is" : "tools are"} missing. Each row above
                says what to install.
              </p>
            </div>
          ) : null}
        </>
      ) : null}
    </Section>
  );
}

function ToolRow({ tool }: { tool: ToolStatus }) {
  const isOptional = tool.requirement === "optional";
  const Icon = tool.found ? CheckIcon : isOptional ? CircleDashedIcon : TriangleAlertIcon;

  return (
    <li className="flex items-start justify-between gap-4 px-4 py-2.5">
      <div className="flex min-w-0 items-start gap-2.5">
        <Icon
          className={cn(
            "mt-0.5 size-4 shrink-0",
            tool.found
              ? "text-success"
              : isOptional
                ? "text-muted-foreground"
                : "text-destructive",
          )}
        />
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium">{tool.name}</span>
            {tool.requirement === "requiredForCuda" ? (
              <Badge variant="outline" className="font-normal">
                CUDA only
              </Badge>
            ) : null}
            {isOptional ? (
              <Badge variant="outline" className="font-normal">
                optional
              </Badge>
            ) : null}
          </span>
          {tool.found ? (
            <span className="truncate text-xs text-muted-foreground">
              {tool.detail}
              {tool.detail && tool.path ? " · " : ""}
              {tool.path ? <code className="font-mono">{tool.path}</code> : null}
            </span>
          ) : (
            <span className="text-xs text-muted-foreground">{tool.remedy}</span>
          )}
        </div>
      </div>

      <span
        className={cn(
          "shrink-0 font-mono text-xs",
          tool.found ? "text-foreground" : "text-muted-foreground",
        )}
      >
        {tool.version ?? (tool.found ? "detected" : "missing")}
      </span>
    </li>
  );
}
