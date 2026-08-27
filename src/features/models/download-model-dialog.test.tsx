import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  inspectMutate: vi.fn(),
  downloadMutate: vi.fn(),
  cancel: vi.fn(),
  inspectPending: false,
  downloadPending: false,
}));

vi.mock("@/hooks/use-models", () => ({
  useInspectHuggingFaceRepository: () => ({
    mutate: mocks.inspectMutate,
    isPending: mocks.inspectPending,
  }),
  useDownloadHuggingFaceModel: () => ({
    mutate: mocks.downloadMutate,
    isPending: mocks.downloadPending,
  }),
}));

vi.mock("@/lib/ipc", () => ({
  createModelDownloadChannel: vi.fn(() => ({ onmessage: null })),
  ipc: { cancelModelDownload: mocks.cancel },
}));

import { DownloadModelDialog } from "./download-model-dialog";

const repository = {
  repositoryId: "owner/coder-GGUF",
  revision: "1234567890abcdef1234567890abcdef12345678",
  selections: [
    {
      id: "coder-q4.gguf",
      displayName: "coder-q4.gguf",
      files: [{ path: "coder-q4.gguf", sizeBytes: 1_000_000 }],
      totalSizeBytes: 1_000_000,
      expectedFiles: 1,
      complete: true,
    },
  ],
};

describe("Hugging Face model download dialog", () => {
  beforeEach(() => {
    mocks.inspectMutate.mockReset();
    mocks.downloadMutate.mockReset();
    mocks.cancel.mockReset();
    mocks.inspectPending = false;
    mocks.downloadPending = false;
  });

  afterEach(cleanup);

  it("loads the exact repository selection and targets a configured model folder", () => {
    mocks.inspectMutate.mockImplementation(
      (_input: string, options: { onSuccess: (value: typeof repository) => void }) => {
        options.onSuccess(repository);
      },
    );
    render(
      <DownloadModelDialog
        open
        directories={["E:\\models", "F:\\models"]}
        onOpenChange={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText("Repository"), {
      target: { value: "owner/coder-GGUF" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Load files" }));

    expect(mocks.inspectMutate).toHaveBeenCalledWith(
      "owner/coder-GGUF",
      expect.any(Object),
    );
    expect(screen.getByText(/pinned to revision 12345678/)).toBeTruthy();

    fireEvent.click(
      screen.getByRole("button", { name: "Download to model folder" }),
    );
    expect(mocks.downloadMutate).toHaveBeenCalledWith(
      expect.objectContaining({
        request: {
          repositoryId: "owner/coder-GGUF",
          revision: repository.revision,
          selectionId: "coder-q4.gguf",
          destinationDirectory: "E:\\models",
        },
      }),
      expect.any(Object),
    );
  });

  it("offers cancellation while a download is active", () => {
    mocks.downloadPending = true;
    mocks.cancel.mockResolvedValue(true);
    render(
      <DownloadModelDialog
        open
        directories={["E:\\models"]}
        onOpenChange={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Cancel download" }));
    expect(mocks.cancel).toHaveBeenCalledTimes(1);
  });
});
