import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { usePreviewVoice } from "./usePreviewVoice";

vi.mock("../backend", () => ({
  previewVoice: vi.fn(async (_provider: string, _voice: string, text: string) =>
    `data:audio/mpeg;base64,mock-${text}`,
  ),
}));

describe("usePreviewVoice", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("starts in idle state", () => {
    const { result } = renderHook(() => usePreviewVoice());
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.audioUrl).toBeNull();
  });

  it("rejects empty text without calling backend", async () => {
    const { result } = renderHook(() => usePreviewVoice());
    await act(async () => {
      await result.current.preview("edge", "en-US-JennyNeural", "");
    });
    expect(result.current.error).toMatch(/Nothing/);
  });

  it("captures error from backend", async () => {
    const { usePreviewVoice } = await import("./usePreviewVoice");
    const backend = await import("../backend");
    (backend.previewVoice as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
      new Error("network unreachable"),
    );
    const { result } = renderHook(() => usePreviewVoice());
    await act(async () => {
      await result.current.preview("edge", "en-US-JennyNeural", "hello");
    });
    await waitFor(() => {
      expect(result.current.error).toMatch(/network/);
    });
  });
});