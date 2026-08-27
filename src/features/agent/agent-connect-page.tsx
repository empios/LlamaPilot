import {
  CableIcon,
  CheckCircle2Icon,
  CopyIcon,
  KeyRoundIcon,
  PlayIcon,
  ServerIcon,
  SlidersHorizontalIcon,
  TerminalIcon,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { ErrorPanel } from "@/components/error-panel";
import { PageHeader } from "@/components/page-header";
import { Section } from "@/components/section";
import { StatGrid, StatTile } from "@/components/stat-tile";
import { StatusChip } from "@/components/status-chip";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
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
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { useTestAgentConnection } from "@/hooks/use-agent";
import { useProfiles } from "@/hooks/use-profiles";
import { useServerStatus, useStartServer } from "@/hooks/use-server";
import { useNavigationStore } from "@/stores/navigation-store";
import type { AgentConnectionTest } from "@/types/agent";
import type { LaunchProfile } from "@/types/profiles";
import { serverIsActive, serverStateLabel } from "@/types/server";

import {
  buildAgentSnippet,
  clientApiBaseUrl,
  profileModelIdentifiers,
  type AgentSnippetKind,
} from "./connection-config";

const SNIPPET_KINDS = ["json", "openaiJs", "aider", "opencode", "pi"] as const;

const SNIPPET_HELP: Record<AgentSnippetKind, string> = {
  json: "Generic connection values for clients with manual OpenAI-compatible setup.",
  openaiJs: "Install the openai package and use this client in a JavaScript project.",
  aider: "Run these commands in PowerShell before starting Aider.",
  opencode: "Save or merge this configuration into opencode.json in your project.",
  pi: "Save or merge this provider into ~/.pi/agent/models.json, then select llamapilot in /model.",
};

export function AgentConnectPage() {
  const profiles = useProfiles();
  const server = useServerStatus();
  const startServer = useStartServer();
  const testConnection = useTestAgentConnection();
  const navigate = useNavigationStore((state) => state.navigate);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [selectedModel, setSelectedModel] = useState("");
  const [testResult, setTestResult] = useState<AgentConnectionTest | null>(null);
  const [snippetKind, setSnippetKind] = useState<AgentSnippetKind>("json");

  const active = server.data ? serverIsActive(server.data.state) : false;
  useEffect(() => {
    const activeProfile = profiles.data?.find(
      (profile) => profile.id === server.data?.profileId,
    );
    if (activeProfile) {
      setSelectedProfileId(activeProfile.id);
    } else if (
      !selectedProfileId ||
      !profiles.data?.some((profile) => profile.id === selectedProfileId)
    ) {
      setSelectedProfileId(profiles.data?.[0]?.id ?? null);
    }
  }, [profiles.data, selectedProfileId, server.data?.profileId]);

  const selectedProfile =
    profiles.data?.find((profile) => profile.id === selectedProfileId) ?? null;
  const activeSelectedProfile =
    active && Boolean(selectedProfile && server.data?.profileId === selectedProfile.id);
  const ready = activeSelectedProfile && server.data?.state === "ready";
  const profileModels = useMemo(
    () => (selectedProfile ? profileModelIdentifiers(selectedProfile) : []),
    [selectedProfile],
  );
  const availableModels = testResult?.modelIds.length
    ? testResult.modelIds
    : profileModels;

  useEffect(() => {
    setSelectedModel((current) =>
      availableModels.includes(current) ? current : (availableModels[0] ?? ""),
    );
  }, [availableModels]);

  useEffect(() => {
    setTestResult(null);
    testConnection.reset();
  }, [selectedProfileId, server.data?.generation]);

  const endpoint = connectionEndpoint(selectedProfile, server.data, activeSelectedProfile);
  const effectiveKey = apiKey.trim() || "no-key";
  const snippet = buildAgentSnippet(snippetKind, {
    apiBaseUrl: endpoint,
    apiKey: effectiveKey,
    model: selectedModel || "model-id",
  });
  const firstError = profiles.error ?? server.error;

  if (profiles.isPending || server.isPending) {
    return (
      <div className="flex flex-col gap-4">
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-72 w-full" />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="Connect"
        title="Agent Connect"
        description="Start one tuned profile, verify its OpenAI-compatible API, and copy the exact connection values into a coding agent."
        actions={
          <Button variant="outline" onClick={() => navigate("profiles")}>
            <SlidersHorizontalIcon data-icon="inline-start" />
            Profiles
          </Button>
        }
      />

      {firstError ? <ErrorPanel error={firstError} /> : null}

      {!firstError && profiles.data?.length === 0 ? (
        <Section label="Setup" title="No launch profile available" icon={CableIcon}>
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <SlidersHorizontalIcon />
              </EmptyMedia>
              <EmptyTitle>Create a profile first</EmptyTitle>
              <EmptyDescription>
                Agent Connect needs a saved runtime, model, and server configuration.
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button onClick={() => navigate("profiles")}>Open Profiles</Button>
            </EmptyContent>
          </Empty>
        </Section>
      ) : null}

      {selectedProfile ? (
        <>
          <div className="flex flex-wrap gap-2">
            <StatusChip
              label="Server"
              value={serverStateLabel(server.data?.state ?? "stopped")}
              tone={ready ? "success" : active ? "warning" : "neutral"}
            />
            <StatusChip label="Profile" value={selectedProfile.name} />
            <StatusChip
              label="API"
              value={testResult ? "verified" : ready ? "ready to test" : "not tested"}
              tone={testResult ? "success" : "neutral"}
            />
          </div>

          <Section
            label="Connection"
            title="OpenAI-compatible endpoint"
            description="The effective port comes from the running supervised process."
            icon={ServerIcon}
            actions={
              !active ? (
                <Button
                  disabled={startServer.isPending}
                  onClick={() => startServer.mutate(selectedProfile.id)}
                >
                  {startServer.isPending ? (
                    <Spinner data-icon="inline-start" />
                  ) : (
                    <PlayIcon data-icon="inline-start" />
                  )}
                  {startServer.isPending ? "Starting…" : "Start profile"}
                </Button>
              ) : null
            }
          >
            <div className="flex flex-col gap-5">
              <StatGrid>
                <StatTile label="API base URL" value={endpoint} mono />
                <StatTile label="Model" value={selectedModel || "Waiting for profile"} mono />
                <StatTile label="Runtime" value={selectedProfile.runtimeLabel} />
                <StatTile
                  label="Compatibility"
                  value={testResult ? "Models + chat passed" : "Pending test"}
                  detail={
                    testResult
                      ? `${formatLatency(testResult.modelsLatencyMs)} discovery · ${formatLatency(testResult.chatLatencyMs)} chat`
                      : "Tests /v1/models and /v1/chat/completions"
                  }
                />
              </StatGrid>

              <FieldGroup>
                <div className="grid gap-4 lg:grid-cols-2">
                  <Field>
                    <FieldLabel>Profile</FieldLabel>
                    <Select
                      value={selectedProfile.id}
                      disabled={active}
                      onValueChange={(value) => setSelectedProfileId(value)}
                    >
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {profiles.data?.map((profile) => (
                          <SelectItem key={profile.id} value={profile.id}>
                            {profile.name} · {profile.modelName}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FieldDescription>
                      Stop the active server before switching to another profile.
                    </FieldDescription>
                  </Field>

                  <Field>
                    <FieldLabel>Model identifier</FieldLabel>
                    <Select
                      value={selectedModel}
                      disabled={availableModels.length < 2}
                      onValueChange={(value) => {
                        setSelectedModel(value);
                        setTestResult(null);
                      }}
                    >
                      <SelectTrigger className="w-full font-mono text-xs">
                        <SelectValue placeholder="Model ID" />
                      </SelectTrigger>
                      <SelectContent>
                        {availableModels.map((model) => (
                          <SelectItem key={model} value={model}>
                            {model}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <div className="flex flex-wrap gap-1.5">
                      {profileModels.map((model) => (
                        <Badge key={model} variant="outline" className="font-mono text-[10px]">
                          {model}
                        </Badge>
                      ))}
                    </div>
                  </Field>
                </div>

                <Field>
                  <FieldLabel htmlFor="agent-api-key">API key</FieldLabel>
                  <div className="flex gap-2">
                    <div className="relative flex-1">
                      <KeyRoundIcon className="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
                      <Input
                        id="agent-api-key"
                        type="password"
                        className="pl-8 font-mono text-xs"
                        value={apiKey}
                        placeholder="Optional — leave empty when llama-server has no API key"
                        onChange={(event) => {
                          setApiKey(event.currentTarget.value);
                          setTestResult(null);
                        }}
                      />
                    </div>
                    <CopyButton value={endpoint} label="Copy URL" />
                    <CopyButton value={selectedModel} label="Copy model" />
                  </div>
                  <FieldDescription>
                    Used for this test and generated snippets only. LlamaPilot does not save it.
                  </FieldDescription>
                </Field>
              </FieldGroup>

              {!active ? (
                <Alert>
                  <ServerIcon />
                  <AlertTitle>Start this profile before testing</AlertTitle>
                  <AlertDescription>
                    The displayed endpoint uses the saved port. If automatic port selection is
                    enabled, it will update after launch.
                  </AlertDescription>
                </Alert>
              ) : !ready ? (
                <Alert>
                  <Spinner />
                  <AlertTitle>Waiting for Ready</AlertTitle>
                  <AlertDescription>
                    The connection test becomes available after the model finishes loading.
                  </AlertDescription>
                </Alert>
              ) : null}

              {testResult ? (
                <Alert className="border-emerald-500/30 bg-emerald-500/5">
                  <CheckCircle2Icon className="text-emerald-600" />
                  <AlertTitle>Agent connection verified</AlertTitle>
                  <AlertDescription>
                    <p>
                      llama-server exposed <strong>{testResult.selectedModel}</strong> and returned
                      a standard assistant message.
                    </p>
                    <code className="mt-2 block rounded bg-background/70 px-2 py-1 font-mono text-xs">
                      {testResult.responseText}
                    </code>
                    {!testResult.requestedModelMatched ? (
                      <p className="mt-2">
                        The configured model name differed from `/v1/models`; snippets now use the
                        identifier reported by the server.
                      </p>
                    ) : null}
                  </AlertDescription>
                </Alert>
              ) : null}

              {testConnection.isError ? <ErrorPanel error={testConnection.error} /> : null}

              <div className="flex justify-end">
                <Button
                  disabled={!ready || testConnection.isPending || !selectedModel}
                  onClick={() =>
                    testConnection.mutate(
                      {
                        apiKey: apiKey.trim() || null,
                        model: selectedModel || null,
                      },
                      {
                        onSuccess: (result) => {
                          setTestResult(result);
                          setSelectedModel(result.selectedModel);
                          toast.success("Agent connection verified");
                        },
                      },
                    )
                  }
                >
                  {testConnection.isPending ? (
                    <Spinner data-icon="inline-start" />
                  ) : (
                    <CableIcon data-icon="inline-start" />
                  )}
                  {testConnection.isPending ? "Testing models and chat…" : "Test connection"}
                </Button>
              </div>
            </div>
          </Section>

          <Section
            label="Client setup"
            title="Copy-ready configuration"
            description="Use the verified model identifier after running the connection test."
            icon={TerminalIcon}
            actions={<CopyButton value={snippet} label="Copy configuration" />}
          >
            <Tabs
              value={snippetKind}
              onValueChange={(value) => setSnippetKind(value as AgentSnippetKind)}
            >
              <TabsList className="h-auto flex-wrap justify-start">
                <TabsTrigger value="json">Connection JSON</TabsTrigger>
                <TabsTrigger value="openaiJs">OpenAI JavaScript</TabsTrigger>
                <TabsTrigger value="aider">Aider PowerShell</TabsTrigger>
                <TabsTrigger value="opencode">OpenCode</TabsTrigger>
                <TabsTrigger value="pi">Pi</TabsTrigger>
              </TabsList>
              {SNIPPET_KINDS.map((kind) => (
                <TabsContent key={kind} value={kind}>
                  <div className="flex flex-col gap-2">
                    <p className="text-xs text-muted-foreground">{SNIPPET_HELP[kind]}</p>
                    <Textarea
                      readOnly
                      value={buildAgentSnippet(kind, {
                        apiBaseUrl: endpoint,
                        apiKey: effectiveKey,
                        model: selectedModel || "model-id",
                      })}
                      className="min-h-64 resize-none font-mono text-xs leading-relaxed"
                      aria-label={`${kind} agent configuration`}
                    />
                  </div>
                </TabsContent>
              ))}
            </Tabs>
          </Section>
        </>
      ) : null}
    </div>
  );
}

function connectionEndpoint(
  profile: LaunchProfile | null,
  snapshot: ReturnType<typeof useServerStatus>["data"],
  activeSelectedProfile: boolean,
): string {
  if (activeSelectedProfile && snapshot?.host && snapshot.port) {
    return clientApiBaseUrl(snapshot.host, snapshot.port);
  }
  return profile ? clientApiBaseUrl(profile.host, profile.port) : "http://127.0.0.1:8080/v1";
}

function formatLatency(milliseconds: number): string {
  return milliseconds < 1_000
    ? `${milliseconds.toFixed(0)} ms`
    : `${(milliseconds / 1_000).toFixed(2)} s`;
}

function CopyButton({ value, label }: { value: string; label: string }) {
  return (
    <Button
      variant="outline"
      disabled={!value}
      onClick={async () => {
        await navigator.clipboard.writeText(value);
        toast.success(`${label} copied`);
      }}
    >
      <CopyIcon data-icon="inline-start" />
      {label}
    </Button>
  );
}
