import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import downloadEvents from "../../../src-tauri/tests/fixtures/model-download-events.json";

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

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage?: (message: unknown) => void;
  },
  invoke: vi.fn(),
  isTauri: () => false,
}));

vi.mock("@/lib/ipc", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/ipc")>()),
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

  it("advances the bar and byte count from backend events through the IPC parser", () => {
    mocks.inspectMutate.mockImplementation(
      (_input: string, options: { onSuccess: (value: typeof repository) => void }) => {
        options.onSuccess(repository);
      },
    );
    const props = {
      open: true,
      directories: ["E:\\models"],
      onOpenChange: vi.fn(),
    };
    const { rerender } = render(<DownloadModelDialog {...props} />);
    fireEvent.change(screen.getByLabelText("Repository"), {
      target: { value: repository.repositoryId },
    });
    fireEvent.click(screen.getByRole("button", { name: "Load files" }));
    fireEvent.click(screen.getByRole("button", { name: "Download to model folder" }));
    const { channel } = mocks.downloadMutate.mock.calls[0]![0] as {
      channel: { onmessage: (message: unknown) => void };
    };
    mocks.downloadPending = true;
    rerender(<DownloadModelDialog {...props} />);

    act(() => {
      channel.onmessage(downloadEvents[0]);
      channel.onmessage(downloadEvents[1]);
    });
    const bar = screen.getByRole("progressbar", { name: "Model download progress" });
    expect(bar.parentElement?.textContent).toContain("coder-q4.gguf");
    for (const [index, percent, bytes] of [
      [2, 25, "244.1 KB / 976.6 KB"],
      [3, 75, "732.4 KB / 976.6 KB"],
      [4, 100, "976.6 KB / 976.6 KB"],
    ] as const) {
      act(() => channel.onmessage(downloadEvents[index]));
      expect(screen.getByText(`${percent}%`)).toBeTruthy();
      expect(screen.getByText(bytes)).toBeTruthy();
      expect(bar.getAttribute("aria-valuenow")).toBe(String(percent));
      expect(bar.querySelector<HTMLElement>("[data-slot='progress-indicator']")?.style.transform)
        .toBe(`translateX(-${100 - percent}%)`);
    }
  });
});
