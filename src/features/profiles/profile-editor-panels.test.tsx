import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { LlamaOption } from "@/types/capabilities";
import type { CommandPreview } from "@/types/profiles";

import {
  ApiModelAliasField,
  CommandPreviewPanel,
  EnvironmentEditor,
} from "./profile-editor-panels";

afterEach(cleanup);

describe("API model alias field", () => {
  const option: LlamaOption = {
    flag: "--alias",
    aliases: ["-a", "--alias"],
    valueHint: "STRING",
    description: "model names used by API",
    section: "server",
    category: "advanced",
    knownKey: "modelAlias",
    displayName: "API model aliases",
    summary: "Names accepted by clients.",
  };

  it("stores comma-separated aliases canonically and clears them with the field", () => {
    const onChange = vi.fn();
    const view = render(
      <ApiModelAliasField
        option={option}
        keyCounts={new Map([["modelAlias", 1]])}
        options={{ "--alias": { mode: "custom", value: "old-name" } }}
        onChange={onChange}
      />,
    );
    const input = screen.getByLabelText("API model aliases");
    expect((input as HTMLInputElement).value).toBe("old-name");

    fireEvent.change(input, { target: { value: "qwen-coder, local-coder" } });
    expect(onChange).toHaveBeenLastCalledWith({
      modelAlias: { mode: "custom", value: "qwen-coder, local-coder" },
    });

    view.rerender(
      <ApiModelAliasField
        option={option}
        keyCounts={new Map([["modelAlias", 1]])}
        options={{ modelAlias: { mode: "custom", value: "qwen-coder" } }}
        onChange={onChange}
      />,
    );
    fireEvent.change(screen.getByLabelText("API model aliases"), { target: { value: "" } });
    expect(onChange).toHaveBeenLastCalledWith({});
  });
});

describe("profile environment editor", () => {
  it("creates a unique variable and preserves its value when renamed", () => {
    const onChange = vi.fn();
    const view = render(
      <EnvironmentEditor environment={{ VARIABLE_1: "one" }} onChange={onChange} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Add variable" }));
    expect(onChange).toHaveBeenLastCalledWith({ VARIABLE_1: "one", VARIABLE_2: "" });

    fireEvent.change(screen.getByLabelText("Environment variable 1 name"), {
      target: { value: "LLAMA_CACHE" },
    });
    expect(onChange).toHaveBeenLastCalledWith({ LLAMA_CACHE: "one" });

    view.rerender(
      <EnvironmentEditor environment={{ LLAMA_CACHE: "one" }} onChange={onChange} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Remove LLAMA_CACHE" }));
    expect(onChange).toHaveBeenLastCalledWith({});
  });

  it("shows an explicit empty state", () => {
    render(<EnvironmentEditor environment={{}} onChange={vi.fn()} />);
    expect(screen.getByText("No environment overrides.")).toBeTruthy();
  });
});

describe("command preview panel", () => {
  const preview: CommandPreview = {
    program: "C:\\Runtime\\llama-server.exe",
    arguments: ["--model", "C:\\Models\\tiny.gguf"],
    environment: { LLAMA_LOG_COLORS: "0" },
    plain: "llama-server.exe --model tiny.gguf",
    powershell: "& 'C:\\Runtime\\llama-server.exe' '--model' 'C:\\Models\\tiny.gguf'",
    runtimeLabel: "CPU · cafebab",
    modelName: "Tiny model",
    capabilityVersion: "b9000-cafebabe",
    warnings: ["The preferred port may be replaced."],
  };

  it("renders command metadata, environment, warnings, and both preview formats", () => {
    render(<CommandPreviewPanel preview={preview} pending={false} error={null} />);

    expect(screen.getByText("CPU · cafebab")).toBeTruthy();
    expect(screen.getByText("LLAMA_LOG_COLORS=0")).toBeTruthy();
    expect(screen.getByText("The preferred port may be replaced.")).toBeTruthy();
    expect(screen.getByText(preview.plain)).toBeTruthy();

    fireEvent.mouseDown(screen.getByRole("tab", { name: "PowerShell" }), {
      button: 0,
      ctrlKey: false,
    });
    expect(screen.getByText(preview.powershell)).toBeTruthy();
  });

  it("explains why no command is available", () => {
    render(<CommandPreviewPanel preview={undefined} pending={false} error={null} />);
    expect(
      screen.getByText("Choose a runtime and complete model to generate the command."),
    ).toBeTruthy();
  });
});
