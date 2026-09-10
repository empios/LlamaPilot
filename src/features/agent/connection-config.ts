import type { LaunchProfile } from "@/types/profiles";

export type AgentSnippetKind = "json" | "openaiJs" | "aiderPosix" | "aider" | "opencode" | "pi";

export interface AgentConnectionValues {
  apiBaseUrl: string;
  apiKey: string;
  model: string;
}

const MODEL_ALIAS_KEYS = new Set(["modelAlias", "--alias", "-a"]);

export function clientApiBaseUrl(host: string, port: number): string {
  const clientHost =
    host === "0.0.0.0" || host === "*"
      ? "127.0.0.1"
      : host === "::" || host === "[::]"
        ? "[::1]"
        : host.includes(":") && !host.startsWith("[")
          ? `[${host}]`
          : host;
  return `http://${clientHost}:${port}/v1`;
}

export function profileModelIdentifiers(profile: LaunchProfile): string[] {
  const aliases = Object.entries(profile.options)
    .filter(([key, setting]) => MODEL_ALIAS_KEYS.has(key) && setting.mode === "custom")
    .flatMap(([, setting]) =>
      setting.mode === "custom" ? setting.value.split(",") : [],
    )
    .map((alias) => alias.trim())
    .filter(Boolean);
  const unique = [...new Set(aliases)];
  return unique.length > 0 ? unique : [profile.modelPath];
}

export function buildAgentSnippet(
  kind: AgentSnippetKind,
  values: AgentConnectionValues,
): string {
  switch (kind) {
    case "json":
      return JSON.stringify(
        {
          baseURL: values.apiBaseUrl,
          apiKey: values.apiKey,
          model: values.model,
        },
        null,
        2,
      );
    case "openaiJs":
      return `import OpenAI from "openai";

const client = new OpenAI({
  baseURL: ${JSON.stringify(values.apiBaseUrl)},
  apiKey: ${JSON.stringify(values.apiKey)},
});

const response = await client.chat.completions.create({
  model: ${JSON.stringify(values.model)},
  messages: [{ role: "user", content: "Review this repository." }],
});

console.log(response.choices[0]?.message?.content);`;
    case "aiderPosix":
      return `env OPENAI_API_BASE=${posixLiteral(values.apiBaseUrl)} OPENAI_API_KEY=${posixLiteral(values.apiKey)} aider --model ${posixLiteral(`openai/${values.model}`)}`;
    case "aider":
      return `$env:OPENAI_API_BASE = '${powershellLiteral(values.apiBaseUrl)}'
$env:OPENAI_API_KEY = '${powershellLiteral(values.apiKey)}'
aider --model 'openai/${powershellLiteral(values.model)}'`;
    case "opencode":
      return JSON.stringify(
        {
          $schema: "https://opencode.ai/config.json",
          model: `llamapilot/${values.model}`,
          provider: {
            llamapilot: {
              npm: "@ai-sdk/openai-compatible",
              name: "LlamaPilot",
              options: {
                baseURL: values.apiBaseUrl,
                apiKey: values.apiKey,
              },
              models: {
                [values.model]: {
                  name: values.model,
                },
              },
            },
          },
        },
        null,
        2,
      );
    case "pi":
      return JSON.stringify(
        {
          providers: {
            llamapilot: {
              baseUrl: values.apiBaseUrl,
              api: "openai-completions",
              apiKey: values.apiKey,
              compat: {
                supportsDeveloperRole: false,
                supportsReasoningEffort: false,
              },
              models: [
                {
                  id: values.model,
                  name: values.model,
                },
              ],
            },
          },
        },
        null,
        2,
      );
  }
}

function powershellLiteral(value: string): string {
  return value.replaceAll("'", "''");
}

function posixLiteral(value: string): string {
  return "'" + value.replaceAll("'", "'\"'\"'") + "'";
}
