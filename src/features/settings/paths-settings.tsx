import {
  DatabaseIcon,
  FolderOpenIcon,
  PlusIcon,
  TrashIcon,
} from "lucide-react";
import { useState } from "react";

import { Section } from "@/components/section";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useAppInfo } from "@/hooks/use-app-info";
import { useDirectoryPicker } from "@/hooks/use-directory-picker";
import { useUpdateSettings } from "@/hooks/use-settings";
import { ipc } from "@/lib/ipc";
import type { Settings } from "@/types/settings";

export function PathsSettings({ settings }: { settings: Settings }) {
  const appInfo = useAppInfo();
  const updateSettings = useUpdateSettings();
  const pickDirectory = useDirectoryPicker();
  const [draft, setDraft] = useState(settings.workspace);

  const applicationPaths = appInfo.data
    ? ([
        ["Data directory", appInfo.data.paths.dataDir],
        ["Settings file", appInfo.data.paths.settingsFile],
        ["Runtimes", appInfo.data.paths.runtimesDir],
        ["Model metadata cache", appInfo.data.paths.modelMetadataCacheFile],
        ["Logs", appInfo.data.paths.logsDir],
      ] as const)
    : [];

  return (
    <div className="flex flex-col gap-6">
      <Section
        label="Workspace"
        title="Where large files live"
        description="Git checkouts and CMake build trees are kept outside the application data directory."
        icon={FolderOpenIcon}
        bodyClassName="p-4"
        actions={
          <Button
            size="sm"
            disabled={updateSettings.isPending}
            onClick={() => updateSettings.mutate({ ...settings, workspace: draft })}
          >
            Save changes
          </Button>
        }
      >
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="sources-directory">llama.cpp sources folder</FieldLabel>
            <div className="flex gap-2">
              <Input
                id="sources-directory"
                value={draft.sourcesDirectory ?? ""}
                placeholder={
                  appInfo.data
                    ? `${appInfo.data.paths.defaultWorkspaceDir}\\sources`
                    : "Default location"
                }
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    sourcesDirectory: event.currentTarget.value || null,
                  })
                }
              />
              <Button
                variant="outline"
                onClick={async () => {
                  const selected = await pickDirectory("Choose a sources folder");
                  if (selected) {
                    setDraft({ ...draft, sourcesDirectory: selected });
                  }
                }}
              >
                <FolderOpenIcon data-icon="inline-start" />
                Browse
              </Button>
            </div>
            <FieldDescription>
              New clones are created inside this folder. Leave it empty to use the default.
            </FieldDescription>
          </Field>

          <Field>
            <FieldLabel htmlFor="builds-directory">Build trees folder</FieldLabel>
            <div className="flex gap-2">
              <Input
                id="builds-directory"
                value={draft.buildsDirectory ?? ""}
                placeholder={
                  appInfo.data
                    ? `${appInfo.data.paths.defaultWorkspaceDir}\\builds`
                    : "Default location"
                }
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    buildsDirectory: event.currentTarget.value || null,
                  })
                }
              />
              <Button
                variant="outline"
                onClick={async () => {
                  const selected = await pickDirectory("Choose a build folder");
                  if (selected) {
                    setDraft({ ...draft, buildsDirectory: selected });
                  }
                }}
              >
                <FolderOpenIcon data-icon="inline-start" />
                Browse
              </Button>
            </div>
            <FieldDescription>
              CMake binary directories are created here, keyed by source, backend, generator,
              and configuration.
            </FieldDescription>
          </Field>

          <Field>
            <div className="flex items-center justify-between gap-3">
              <div>
                <FieldLabel>GGUF model folders</FieldLabel>
                <FieldDescription>
                  Every folder is scanned recursively. Tensor data is never loaded while listing.
                </FieldDescription>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={async () => {
                  const selected = await pickDirectory("Choose a GGUF model folder");
                  if (selected && !draft.modelDirectories.includes(selected)) {
                    setDraft({
                      ...draft,
                      modelDirectories: [...draft.modelDirectories, selected],
                    });
                  }
                }}
              >
                <PlusIcon data-icon="inline-start" />
                Add folder
              </Button>
            </div>

            {draft.modelDirectories.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border px-4 py-3 text-sm text-muted-foreground">
                No model folders configured. Add one to populate the Models page.
              </div>
            ) : (
              <div className="flex flex-col gap-2">
                {draft.modelDirectories.map((directory, index) => (
                  <div key={`${directory}-${index}`} className="flex gap-2">
                    <Input
                      aria-label={`Model folder ${index + 1}`}
                      value={directory}
                      onChange={(event) => {
                        const modelDirectories = [...draft.modelDirectories];
                        modelDirectories[index] = event.currentTarget.value;
                        setDraft({ ...draft, modelDirectories });
                      }}
                    />
                    <Button
                      variant="outline"
                      size="icon"
                      onClick={async () => {
                        const selected = await pickDirectory("Choose a GGUF model folder");
                        if (selected) {
                          const modelDirectories = [...draft.modelDirectories];
                          modelDirectories[index] = selected;
                          setDraft({ ...draft, modelDirectories });
                        }
                      }}
                    >
                      <FolderOpenIcon />
                      <span className="sr-only">Browse for model folder {index + 1}</span>
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() =>
                        setDraft({
                          ...draft,
                          modelDirectories: draft.modelDirectories.filter(
                            (_, itemIndex) => itemIndex !== index,
                          ),
                        })
                      }
                    >
                      <TrashIcon />
                      <span className="sr-only">Remove model folder {index + 1}</span>
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </Field>
        </FieldGroup>
      </Section>

      <Section
        label="Application data"
        title="Managed by Llama Control"
        description="Settings, the source registry, runtimes, and logs."
        icon={DatabaseIcon}
        bodyClassName="p-0"
      >
        <ul className="divide-y divide-border">
          {applicationPaths.map(([label, value]) => (
            <li
              key={label}
              className="flex items-center justify-between gap-4 px-4 py-2.5"
            >
              <div className="flex min-w-0 flex-col">
                <span className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
                  {label}
                </span>
                <code className="truncate font-mono text-xs">{value}</code>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => void ipc.revealPath(value)}
              >
                <FolderOpenIcon data-icon="inline-start" />
                Open
              </Button>
            </li>
          ))}
        </ul>
      </Section>
    </div>
  );
}
