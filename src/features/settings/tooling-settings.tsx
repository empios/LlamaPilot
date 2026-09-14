import { useUpdateBlocker } from "@/hooks/use-update-blocker";
import { useState } from "react";

import { WrenchIcon } from "lucide-react";

import { Section } from "@/components/section";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useUpdateSettings } from "@/hooks/use-settings";
import type { Settings } from "@/types/settings";

export function ToolingSettings({ settings }: { settings: Settings }) {
  const updateSettings = useUpdateSettings();
  const [git, setGit] = useState(settings.git);
  const [build, setBuild] = useState(settings.build);

  useUpdateBlocker(JSON.stringify(git) !== JSON.stringify(settings.git) || JSON.stringify(build) !== JSON.stringify(settings.build));

  return (
    <Section
      label="Tooling"
      title="External tools"
      description="Leave an executable field empty to resolve the tool from PATH."
      icon={WrenchIcon}
      bodyClassName="p-4"
      actions={
        <Button
          size="sm"
          disabled={updateSettings.isPending}
          onClick={() => updateSettings.mutate({ ...settings, git, build })}
        >
          Save changes
        </Button>
      }
    >
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="default-repository">
            Default llama.cpp repository
          </FieldLabel>
          <Input
            id="default-repository"
            value={git.defaultRepository}
            onChange={(event) =>
              setGit({ ...git, defaultRepository: event.currentTarget.value })
            }
          />
          <FieldDescription>
            Pre-filled when adding a source. Change it to clone a fork by
            default.
          </FieldDescription>
        </Field>

        <Field>
          <FieldLabel htmlFor="git-executable">Git executable</FieldLabel>
          <Input
            id="git-executable"
            value={git.executable ?? ""}
            placeholder="Resolved from PATH"
            onChange={(event) =>
              setGit({ ...git, executable: event.currentTarget.value || null })
            }
          />
        </Field>

        <Field>
          <FieldLabel htmlFor="cmake-executable">CMake executable</FieldLabel>
          <Input
            id="cmake-executable"
            value={build.cmakeExecutable ?? ""}
            placeholder="Resolved from PATH"
            onChange={(event) =>
              setBuild({
                ...build,
                cmakeExecutable: event.currentTarget.value || null,
              })
            }
          />
          <FieldDescription>
            Visual Studio installs CMake alongside the C++ workload; point here
            to use a specific copy.
          </FieldDescription>
        </Field>

        <Field>
          <FieldLabel htmlFor="parallel-jobs">Parallel build jobs</FieldLabel>
          <Input
            id="parallel-jobs"
            type="number"
            min={1}
            max={256}
            value={build.parallelJobs ?? ""}
            placeholder="Let CMake decide"
            onChange={(event) => {
              const raw = event.currentTarget.value;
              setBuild({
                ...build,
                parallelJobs: raw === "" ? null : Number(raw),
              });
            }}
          />
          <FieldDescription>
            Passed to <code>cmake --build --parallel</code>. Empty means CMake
            picks the job count itself.
          </FieldDescription>
        </Field>
      </FieldGroup>
    </Section>
  );
}
