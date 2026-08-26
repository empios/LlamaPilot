import { PageHeader } from "@/components/page-header";
import { BuildForm } from "@/features/builds/build-form";
import { RuntimeHistory } from "@/features/builds/runtime-history";
import { ToolchainPanel } from "@/features/builds/toolchain-panel";

export function BuildPage() {
  return (
    <div className="flex flex-col gap-7">
      <PageHeader
        eyebrow="llama.cpp"
        title="Build"
        description="Compile llama-server from a registered source and snapshot it into an immutable runtime."
      />

      <ToolchainPanel />
      <BuildForm />
      <RuntimeHistory />
    </div>
  );
}
