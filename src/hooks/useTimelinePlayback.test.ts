import { describe, expect, it } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useTimelinePlayback } from "./useTimelinePlayback";
import type { Scene } from "../types";

const scene = (id: string, duration: number, selected = true): Scene => ({
  id,
  page: 1,
  title: id,
  script: "x",
  duration,
  selected,
  thumbnail: "",
});

describe("useTimelinePlayback", () => {
  it("starts in stopped state", () => {
    const { result } = renderHook(() => useTimelinePlayback([scene("a", 5)]));
    expect(result.current.playing).toBe(false);
    expect(result.current.currentSceneIndex).toBe(0);
    expect(result.current.totalElapsed).toBe(0);
    expect(result.current.totalDuration).toBe(5);
  });

  it("transitions to playing on play()", () => {
    const { result } = renderHook(() => useTimelinePlayback([scene("a", 5), scene("b", 5)]));
    act(() => result.current.play());
    expect(result.current.playing).toBe(true);
  });

  it("computes total duration from selected scenes only", () => {
    const scenes = [scene("a", 5, true), scene("b", 10, false), scene("c", 3, true)];
    const { result } = renderHook(() => useTimelinePlayback(scenes));
    expect(result.current.totalDuration).toBe(8);
  });

  it("does not play when no scenes selected", () => {
    const scenes = [scene("a", 5, false)];
    const { result } = renderHook(() => useTimelinePlayback(scenes));
    act(() => result.current.play());
    expect(result.current.playing).toBe(false);
  });

  it("stops on stop()", () => {
    const { result } = renderHook(() => useTimelinePlayback([scene("a", 5)]));
    act(() => result.current.play());
    act(() => result.current.stop());
    expect(result.current.playing).toBe(false);
    expect(result.current.currentSceneIndex).toBe(0);
  });
});