import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Scene } from "../types";
import { useTimelinePlayback } from "./useTimelinePlayback";

const scene = (id: string, duration: number, selected = true): Scene => ({
  id,
  page: 1,
  title: id,
  script: "x",
  duration,
  selected,
  thumbnail: "",
});

// New signature: useTimelinePlayback(scenes, activeId, onActiveChange).
// `activeId` is the single source of truth for which scene is current.
const render = (scenes: Scene[], activeId: string | null = null) =>
  renderHook(() => useTimelinePlayback(scenes, activeId, vi.fn()));

describe("useTimelinePlayback", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("starts in stopped state", () => {
    const { result } = render([scene("a", 5)], "a");
    expect(result.current.playing).toBe(false);
    expect(result.current.currentSceneIndex).toBe(0);
    expect(result.current.totalElapsed).toBe(0);
    expect(result.current.totalDuration).toBe(5);
  });

  it("transitions to playing on play()", () => {
    const { result } = render([scene("a", 5), scene("b", 5)], "a");
    act(() => result.current.play());
    expect(result.current.playing).toBe(true);
  });

  it("computes total duration from selected scenes only", () => {
    const scenes = [scene("a", 5, true), scene("b", 10, false), scene("c", 3, true)];
    const { result } = render(scenes, "a");
    expect(result.current.totalDuration).toBe(8);
  });

  it("does not play when no scenes selected", () => {
    const scenes = [scene("a", 5, false)];
    const { result } = render(scenes, "a");
    act(() => result.current.play());
    expect(result.current.playing).toBe(false);
  });

  it("advances activeId to the next scene while playing", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTimelinePlayback([scene("a", 1), scene("b", 1)], "a", onActiveChange),
    );
    act(() => result.current.play());
    // Advance past the first scene's duration (tick is 200ms; duration=1s).
    act(() => {
      vi.advanceTimersByTime(1200);
    });
    expect(onActiveChange).toHaveBeenCalledWith("b");
  });

  it("resumes on the same scene after pause, instead of restarting the document", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTimelinePlayback([scene("a", 5), scene("b", 5)], "b", onActiveChange),
    );
    act(() => result.current.pause());
    onActiveChange.mockClear();
    act(() => result.current.play());
    expect(result.current.playing).toBe(true);
    // Resume must NOT jump back to the first scene.
    expect(onActiveChange).not.toHaveBeenCalled();
  });

  it("restarts from the first selected scene after stop()", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTimelinePlayback([scene("a", 5), scene("b", 5)], "b", onActiveChange),
    );
    act(() => result.current.stop());
    act(() => result.current.play());
    expect(onActiveChange).toHaveBeenCalledWith("a");
  });

  it("stops on stop()", () => {
    const { result } = render([scene("a", 5)], "a");
    act(() => result.current.play());
    act(() => result.current.stop());
    expect(result.current.playing).toBe(false);
    expect(result.current.currentSceneIndex).toBe(0);
  });
});
