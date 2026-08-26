import { FolderOpenIcon } from "lucide-react";
import { useState } from "react";

import { OutputConsole, appendProgressEvent, type ConsoleLine } from "@/components/output-console";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useDirectoryPicker } from "@/hooks/use-directory-picker";
import { useSettings } from "@/hooks/use-settings";
import { useAddExistingSource, useCloneSource } from "@/hooks/use-sources";
import { createProgressChannel } from "@/lib/ipc";
import { branding } from "@/lib/branding";

interface AddSourceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function AddSourceDialog({ open, onOpenChange }: AddSourceDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Add llama.cpp source</DialogTitle>
          <DialogDescription>
            Clone a repository, or register a checkout you already have on disk.
          </DialogDescription>
        </DialogHeader>

        <Tabs defaultValue="clone">
          <TabsList>
            <TabsTrigger value="clone">Clone</TabsTrigger>
            <TabsTrigger value="existing">Use existing folder</TabsTrigger>
          </TabsList>
          <TabsContent value="clone">
            <CloneForm onDone={() => onOpenChange(false)} />
          </TabsContent>
          <TabsContent value="existing">
            <ExistingForm onDone={() => onOpenChange(false)} />
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}

function CloneForm({ onDone }: { onDone: () => void }) {
  const settings = useSettings();
  const pickDirectory = useDirectoryPicker();
  const cloneSource = useCloneSource();

  const [repository, setRepository] = useState("");
  const [destinationParent, setDestinationParent] = useState("");
  const [directoryName, setDirectoryName] = useState("");
  const [branch, setBranch] = useState("");
  const [lines, setLines] = useState<ConsoleLine[]>([]);

  const effectiveRepository =
    repository.trim() || settings.data?.git.defaultRepository || branding.upstreamRepository;

  const startClone = () => {
    setLines([]);
    const channel = createProgressChannel((event) => {
      setLines((current) => appendProgressEvent(current, event));
    });

    cloneSource.mutate(
      {
        request: {
          repository: effectiveRepository,
          destinationParent: destinationParent.trim() || null,
          directoryName: directoryName.trim() || null,
          branch: branch.trim() || null,
        },
        channel,
      },
      { onSuccess: onDone },
    );
  };

  return (
    <div className="flex flex-col gap-4 pt-2">
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="clone-repository">Repository</FieldLabel>
          <Input
            id="clone-repository"
            value={repository}
            placeholder={settings.data?.git.defaultRepository ?? branding.upstreamRepository}
            onChange={(event) => setRepository(event.currentTarget.value)}
          />
          <FieldDescription>
            Any Git URL works, so a fork carrying experimental backends can be used directly.
          </FieldDescription>
        </Field>

        <Field>
          <FieldLabel htmlFor="clone-parent">Destination folder</FieldLabel>
          <div className="flex gap-2">
            <Input
              id="clone-parent"
              value={destinationParent}
              placeholder="Configured sources folder"
              onChange={(event) => setDestinationParent(event.currentTarget.value)}
            />
            <Button
              variant="outline"
              onClick={async () => {
                const selected = await pickDirectory("Choose where to clone");
                if (selected) {
                  setDestinationParent(selected);
                }
              }}
            >
              <FolderOpenIcon data-icon="inline-start" />
              Browse
            </Button>
          </div>
        </Field>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field>
            <FieldLabel htmlFor="clone-folder">Folder name</FieldLabel>
            <Input
              id="clone-folder"
              value={directoryName}
              placeholder="Derived from the repository"
              onChange={(event) => setDirectoryName(event.currentTarget.value)}
            />
          </Field>

          <Field>
            <FieldLabel htmlFor="clone-branch">Branch</FieldLabel>
            <Input
              id="clone-branch"
              value={branch}
              placeholder="Repository default"
              onChange={(event) => setBranch(event.currentTarget.value)}
            />
            <FieldDescription>
              Empty clones the branch the remote reports as its default.
            </FieldDescription>
          </Field>
        </div>
      </FieldGroup>

      {lines.length > 0 ? <OutputConsole lines={lines} /> : null}

      <DialogFooter>
        <Button onClick={startClone} disabled={cloneSource.isPending}>
          {cloneSource.isPending ? <Spinner data-icon="inline-start" /> : null}
          {cloneSource.isPending ? "Cloning…" : "Clone repository"}
        </Button>
      </DialogFooter>
    </div>
  );
}

function ExistingForm({ onDone }: { onDone: () => void }) {
  const pickDirectory = useDirectoryPicker();
  const addExistingSource = useAddExistingSource();

  const [directory, setDirectory] = useState("");
  const [name, setName] = useState("");

  return (
    <div className="flex flex-col gap-4 pt-2">
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="existing-directory">llama.cpp folder</FieldLabel>
          <div className="flex gap-2">
            <Input
              id="existing-directory"
              value={directory}
              placeholder="C:\repos\llama.cpp"
              onChange={(event) => setDirectory(event.currentTarget.value)}
            />
            <Button
              variant="outline"
              onClick={async () => {
                const selected = await pickDirectory("Choose an existing llama.cpp checkout");
                if (selected) {
                  setDirectory(selected);
                }
              }}
            >
              <FolderOpenIcon data-icon="inline-start" />
              Browse
            </Button>
          </div>
          <FieldDescription>
            Must be a Git working copy. Nothing inside it is modified when you register it.
          </FieldDescription>
        </Field>

        <Field>
          <FieldLabel htmlFor="existing-name">Display name</FieldLabel>
          <Input
            id="existing-name"
            value={name}
            placeholder="Derived from the remote URL"
            onChange={(event) => setName(event.currentTarget.value)}
          />
        </Field>
      </FieldGroup>

      <DialogFooter>
        <Button
          disabled={!directory.trim() || addExistingSource.isPending}
          onClick={() =>
            addExistingSource.mutate(
              { directory: directory.trim(), name: name.trim() || undefined },
              { onSuccess: onDone },
            )
          }
        >
          {addExistingSource.isPending ? <Spinner data-icon="inline-start" /> : null}
          Add source
        </Button>
      </DialogFooter>
    </div>
  );
}
