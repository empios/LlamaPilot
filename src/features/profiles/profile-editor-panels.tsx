import {
  AlertTriangleIcon,
  CopyIcon,
  PlusIcon,
  RefreshCwIcon,
  SlidersHorizontalIcon,
  TrashIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { ErrorPanel } from "@/components/error-panel";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { LlamaOption } from "@/types/capabilities";
import type { CommandPreview, ProfileOptionSetting } from "@/types/profiles";

import {
  profileOptionKey,
  settingForOption,
  updateProfileOption,
} from "./profile-option-controls";

export function ApiModelAliasField({
  option,
  keyCounts,
  options,
  onChange,
}: {
  option: LlamaOption;
  keyCounts: Map<string, number>;
  options: Record<string, ProfileOptionSetting>;
  onChange: (options: Record<string, ProfileOptionSetting>) => void;
}) {
  const key = profileOptionKey(option, keyCounts);
  const setting = settingForOption(options, option, key);
  const value = setting.mode === "custom" ? setting.value : "";

  return (
    <Field>
      <FieldLabel htmlFor="profile-model-alias">API model aliases</FieldLabel>
      <Input
        id="profile-model-alias"
        className="font-mono text-xs"
        value={value}
        placeholder="qwen-coder, local-coder"
        onChange={(event) =>
          onChange(
            updateProfileOption(
              options,
              option,
              key,
              event.currentTarget.value
                ? { mode: "custom", value: event.currentTarget.value }
                : { mode: "default" },
            ),
          )
        }
      />
      <FieldDescription>
        Optional comma-separated names that coding agents can send in the OpenAI-compatible API
        <code className="ml-1 font-mono text-[11px]">model</code> field.
      </FieldDescription>
    </Field>
  );
}

export function EnvironmentEditor({
  environment,
  onChange,
}: {
  environment: Record<string, string>;
  onChange: (environment: Record<string, string>) => void;
}) {
  const entries = Object.entries(environment);
  const addVariable = () => {
    let index = entries.length + 1;
    let key = `VARIABLE_${index}`;
    while (Object.hasOwn(environment, key)) {
      key = `VARIABLE_${++index}`;
    }
    onChange({ ...environment, [key]: "" });
  };

  return (
    <FieldGroup>
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-medium">Per-process environment</p>
          <p className="text-xs text-muted-foreground">
            These values are attached only to llama-server and never written to the global
            environment.
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={addVariable}>
          <PlusIcon data-icon="inline-start" />
          Add variable
        </Button>
      </div>
      {entries.length === 0 ? (
        <div className="rounded-lg border border-dashed p-5 text-center text-sm text-muted-foreground">
          No environment overrides.
        </div>
      ) : (
        entries.map(([key, value], index) => (
          <div
            key={`${index}-${key}`}
            className="grid gap-2 md:grid-cols-[minmax(160px,0.7fr)_1fr_auto]"
          >
            <Input
              aria-label={`Environment variable ${index + 1} name`}
              className="font-mono text-xs"
              value={key}
              onChange={(event) => {
                const next = Object.fromEntries(
                  entries.map(([entryKey, entryValue], entryIndex) =>
                    entryIndex === index
                      ? [event.currentTarget.value, entryValue]
                      : [entryKey, entryValue],
                  ),
                );
                onChange(next);
              }}
            />
            <Input
              aria-label={`Environment variable ${key} value`}
              className="font-mono text-xs"
              value={value}
              onChange={(event) =>
                onChange({ ...environment, [key]: event.currentTarget.value })
              }
            />
            <Button
              variant="ghost"
              size="icon"
              onClick={() =>
                onChange(
                  Object.fromEntries(
                    entries.filter((_, entryIndex) => entryIndex !== index),
                  ),
                )
              }
            >
              <TrashIcon />
              <span className="sr-only">Remove {key}</span>
            </Button>
          </div>
        ))
      )}
    </FieldGroup>
  );
}

export function CommandPreviewPanel({
  preview,
  pending,
  error,
}: {
  preview: CommandPreview | undefined;
  pending: boolean;
  error: unknown;
}) {
  const [format, setFormat] = useState("command");
  const content = format === "powershell" ? preview?.powershell : preview?.plain;
  return (
    <section className="flex min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-card">
      <header className="flex items-center justify-between gap-3 border-b border-border px-3 py-2.5">
        <div>
          <p className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
            Generated command
          </p>
          <p className="text-xs text-muted-foreground">
            Display-only; execution uses the argument array.
          </p>
        </div>
        {pending ? <RefreshCwIcon className="size-4 animate-spin text-muted-foreground" /> : null}
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto p-3">
        {!preview && pending ? <Skeleton className="h-48 w-full" /> : null}
        {error ? <ErrorPanel error={error} /> : null}
        {!preview && !pending && !error ? (
          <div className="flex min-h-48 flex-col items-center justify-center gap-2 rounded-lg border border-dashed p-5 text-center text-sm text-muted-foreground">
            <SlidersHorizontalIcon className="size-5" />
            Choose a runtime and complete model to generate the command.
          </div>
        ) : null}
        {preview ? (
          <div className="space-y-3">
            <div className="flex flex-wrap gap-1.5">
              <Badge variant="secondary">{preview.runtimeLabel}</Badge>
              <Badge variant="outline">{preview.modelName}</Badge>
              <Badge variant="outline">{preview.capabilityVersion}</Badge>
            </div>
            <Tabs value={format} onValueChange={setFormat}>
              <div className="flex items-center justify-between gap-2">
                <TabsList>
                  <TabsTrigger value="command">Command</TabsTrigger>
                  <TabsTrigger value="powershell">PowerShell</TabsTrigger>
                </TabsList>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => content && void copyText(content)}
                >
                  <CopyIcon data-icon="inline-start" />
                  Copy
                </Button>
              </div>
              <TabsContent value="command">
                <pre className="mt-2 max-h-72 overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap break-all">
                  {preview.plain}
                </pre>
              </TabsContent>
              <TabsContent value="powershell">
                <pre className="mt-2 max-h-72 overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap break-all">
                  {preview.powershell}
                </pre>
              </TabsContent>
            </Tabs>
            {Object.keys(preview.environment).length > 0 ? (
              <div>
                <p className="mb-1 text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
                  Environment
                </p>
                {Object.entries(preview.environment).map(([key, value]) => (
                  <code key={key} className="block truncate font-mono text-xs">
                    {key}={value}
                  </code>
                ))}
              </div>
            ) : null}
            {preview.warnings.map((warning) => (
              <Alert key={warning}>
                <AlertTriangleIcon />
                <AlertTitle>Launch-time adjustment</AlertTitle>
                <AlertDescription>{warning}</AlertDescription>
              </Alert>
            ))}
          </div>
        ) : null}
      </div>
    </section>
  );
}

async function copyText(value: string) {
  try {
    await navigator.clipboard.writeText(value);
    toast.success("Command copied");
  } catch (error) {
    toast.error("Could not copy the command", {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}
