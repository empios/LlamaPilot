import { PlayIcon, RotateCwIcon, ServerIcon, SquareIcon } from "lucide-react";

import { Section } from "@/components/section";
import { StatGrid, StatTile } from "@/components/stat-tile";
import { Button } from "@/components/ui/button";
import { useModels } from "@/hooks/use-models";
import { useRuntimes } from "@/hooks/use-build";
import { useProfiles } from "@/hooks/use-profiles";
import { useSources } from "@/hooks/use-sources";
import { useNavigationStore } from "@/stores/navigation-store";
import {
  useRestartServer,
  useServerStatus,
  useStartServer,
  useStopServer,
} from "@/hooks/use-server";
import { serverIsActive, serverStateLabel, type ServerSnapshot } from "@/types/server";

export function ServerPanel() {
  const sources = useSources();
  const models = useModels();
  const runtimes = useRuntimes();
  const profiles = useProfiles();
  const navigate = useNavigationStore((state) => state.navigate);
  const server = useServerStatus();
  const startServer = useStartServer();
  const stopServer = useStopServer();
  const restartServer = useRestartServer();

  const hasSource = (sources.data?.length ?? 0) > 0;
  const runtimeCount = runtimes.data?.length ?? 0;
  const modelCount = models.data?.models.filter((model) => model.complete).length ?? 0;
  const profileCount = profiles.data?.length ?? 0;
  const snapshot = server.data;
  const active = snapshot ? serverIsActive(snapshot.state) : false;
  const nextStep = !hasSource
    ? { value: "Add a source", label: "Open Runtimes", page: "runtimes" as const }
    : runtimeCount === 0
      ? { value: "Build llama.cpp", label: "Open Build", page: "build" as const }
      : modelCount === 0
        ? { value: "Add a model", label: "Open Models", page: "models" as const }
        : profileCount === 0
          ? { value: "Create a profile", label: "Open Profiles", page: "profiles" as const }
          : { value: "Ready for agent", label: "Open Agent Connect", page: "agent" as const };

  return (
    <Section
      label="Server"
      title="llama-server"
      icon={ServerIcon}
      actions={
        active ? (
          <>
            <Button
              size="sm"
              variant="outline"
              disabled={restartServer.isPending || snapshot?.state === "stopping"}
              onClick={() => restartServer.mutate()}
            >
              <RotateCwIcon data-icon="inline-start" />
              Restart
            </Button>
            <Button
              size="sm"
              disabled={stopServer.isPending || snapshot?.state === "stopping"}
              onClick={() => stopServer.mutate()}
            >
              <SquareIcon data-icon="inline-start" />
              Stop
            </Button>
          </>
        ) : (
          <Button
            size="sm"
            disabled={!profiles.data?.[0] || startServer.isPending}
            onClick={() => profiles.data?.[0] && startServer.mutate(profiles.data[0].id)}
          >
            <PlayIcon data-icon="inline-start" />
            Start server
          </Button>
        )
      }
    >
      <StatGrid>
        <StatTile
          label="Status"
          value={server.isPending ? "Loading…" : serverStateLabel(snapshot?.state ?? "stopped")}
          detail={snapshot?.healthMessage ?? snapshot?.lastError ?? (profileCount > 0 ? "Ready to launch" : "Complete the setup steps")}
        />
        <StatTile
          label="Profile"
          value={snapshot?.profileName ?? "None selected"}
          detail={snapshot?.modelName ?? (runtimeCount > 0 ? `${runtimeCount} runtimes available` : "Build a runtime first")}
        />
        <StatTile
          label="Endpoint"
          value={snapshot?.host && snapshot.port ? `${snapshot.host}:${snapshot.port}` : "Not listening"}
          detail={
            <Button
              variant="link"
              size="sm"
              className="h-auto p-0 text-xs"
              onClick={() => navigate(active ? "logs" : "profiles")}
            >
              {active ? "Open live logs" : "Choose a profile"}
            </Button>
          }
        />
        <StatTile
          label={active ? "Activity" : "Next step"}
          value={active ? activityValue(snapshot) : nextStep.value}
          detail={
            active ? (
              `${snapshot?.telemetry.busySlots ?? 0}/${snapshot?.telemetry.totalSlots ?? "?"} slots busy`
            ) : (
              <Button
                variant="link"
                size="sm"
                className="h-auto p-0 text-xs"
                onClick={() => navigate(nextStep.page)}
              >
                {nextStep.label}
              </Button>
            )
          }
        />
      </StatGrid>
    </Section>
  );
}

function activityValue(snapshot: ServerSnapshot | undefined): string {
  const tokens = snapshot?.telemetry.predictedTokensPerSecond;
  if (tokens !== null && tokens !== undefined) return `${tokens.toFixed(1)} tok/s`;
  const processing = snapshot?.telemetry.requestsProcessing;
  if (processing !== null && processing !== undefined) return `${processing} processing`;
  return snapshot?.state === "busy" ? "Handling request" : "Idle";
}
