import { useUpdateBlocker } from "@/hooks/use-update-blocker";
import { useState } from "react";

import { ServerIcon } from "lucide-react";

import { Section } from "@/components/section";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { useUpdateSettings } from "@/hooks/use-settings";
import type { Settings } from "@/types/settings";

export function GeneralSettings({ settings }: { settings: Settings }) {
  const updateSettings = useUpdateSettings();
  const [draft, setDraft] = useState(settings.server);
  const [compact, setCompact] = useState(settings.appearance.compactDensity);

  useUpdateBlocker(JSON.stringify(draft) !== JSON.stringify(settings.server) || compact !== settings.appearance.compactDensity);

  const portIsValid = draft.defaultPort >= 1 && draft.defaultPort <= 65535;

  return (
    <Section
      label="Defaults"
      title="Server defaults"
      description="Starting values for new profiles. A profile can always override them."
      icon={ServerIcon}
      bodyClassName="flex flex-col gap-6 p-4"
      actions={
        <Button
          size="sm"
          disabled={!portIsValid || updateSettings.isPending}
          onClick={() =>
            updateSettings.mutate({
              ...settings,
              server: draft,
              appearance: { ...settings.appearance, compactDensity: compact },
            })
          }
        >
          Save changes
        </Button>
      }
    >
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="default-host">Default host</FieldLabel>
          <Input
            id="default-host"
            value={draft.defaultHost}
            onChange={(event) =>
              setDraft({ ...draft, defaultHost: event.currentTarget.value })
            }
          />
          <FieldDescription>
            Passed to llama-server as <code>--host</code>. Keep it on 127.0.0.1
            unless you intend to expose the server on your network.
          </FieldDescription>
        </Field>

        <Field data-invalid={portIsValid ? undefined : true}>
          <FieldLabel htmlFor="default-port">Default port</FieldLabel>
          <Input
            id="default-port"
            type="number"
            min={1}
            max={65535}
            aria-invalid={portIsValid ? undefined : true}
            value={draft.defaultPort}
            onChange={(event) =>
              setDraft({
                ...draft,
                defaultPort: Number(event.currentTarget.value),
              })
            }
          />
          <FieldDescription>
            {portIsValid
              ? "Passed to llama-server as --port."
              : "Enter a port between 1 and 65535."}
          </FieldDescription>
        </Field>

        <Field orientation="horizontal">
          <Switch
            id="auto-select-port"
            checked={draft.autoSelectPort}
            onCheckedChange={(checked) =>
              setDraft({ ...draft, autoSelectPort: checked })
            }
          />
          <FieldLabel htmlFor="auto-select-port">
            Pick a free port automatically when the default is in use
          </FieldLabel>
        </Field>

        <Field orientation="horizontal">
          <Switch
            id="compact-density"
            checked={compact}
            onCheckedChange={setCompact}
          />
          <FieldLabel htmlFor="compact-density">
            Compact layout density
          </FieldLabel>
        </Field>
      </FieldGroup>
    </Section>
  );
}
