import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Scene } from "../types";

interface PlaybackState {
  playing: boolean;
  elapsedInScene: number;
}

interface TimelinePlayback {
  playing: boolean;
  /** Index of the active scene within the selected-scenes list. */
  currentSceneIndex: number;
  elapsedInScene: number;
  /** Elapsed seconds across the whole selected timeline. */
  totalElapsed: number;
  totalDuration: number;
  play: () => void;
  pause: () => void;
  stop: () => void;
}

/**
 * Timeline playback driven by the single source of truth: `activeId`.
 *
 * The hook does not keep its own copy of "which scene is current" — it
 * derives that from `activeId` so the preview image, caption, timeline
 * highlight and audio all stay in lock-step with whatever the user
 * clicked. While playing, it advances by calling `onActiveChange` with
 * the next selected scene's id; the parent updates `activeId`, which
 * feeds back in and keeps `currentSceneIndex` correct.
 */
export function useTimelinePlayback(
  scenes: Scene[],
  activeId: string | null,
  onActiveChange: (sceneId: string) => void,
): TimelinePlayback {
  const [state, setState] = useState<PlaybackState>({
    playing: false,
    elapsedInScene: 0,
  });
  const timerRef = useRef<number | null>(null);

  const selectedScenes = useMemo(() => scenes.filter((s) => s.selected), [scenes]);
  const totalDuration = selectedScenes.reduce((sum, scene) => sum + scene.duration, 0);

  const currentSceneIndex = Math.max(
    0,
    selectedScenes.findIndex((s) => s.id === activeId),
  );

  const totalElapsed =
    selectedScenes.slice(0, currentSceneIndex).reduce((sum, s) => sum + s.duration, 0) +
    state.elapsedInScene;

  const clearTimer = useCallback(() => {
    if (timerRef.current) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  // Keep the latest values in refs so the interval closure never goes
  // stale and never calls a parent setter from inside a setState updater.
  const selectedRef = useRef(selectedScenes);
  const activeIdRef = useRef(activeId);
  const onActiveChangeRef = useRef(onActiveChange);
  selectedRef.current = selectedScenes;
  activeIdRef.current = activeId;
  onActiveChangeRef.current = onActiveChange;

  // Distinguishes pause (resume where you were) from stop / first play
  // (start the document from the beginning).
  const pausedRef = useRef(false);

  const play = useCallback(() => {
    if (selectedScenes.length === 0) return;
    if (pausedRef.current && selectedScenes.some((s) => s.id === activeId)) {
      // Resume after pause: stay on the current scene. The scene restarts
      // from its beginning (elapsed 0) because the narration audio cannot
      // resume mid-utterance — restarting keeps timer and voice in sync.
      pausedRef.current = false;
      setState({ playing: true, elapsedInScene: 0 });
      return;
    }
    // Fresh start (or the paused scene was deselected): read the document
    // from the first selected page.
    pausedRef.current = false;
    onActiveChange(selectedScenes[0].id);
    setState({ playing: true, elapsedInScene: 0 });
  }, [selectedScenes, activeId, onActiveChange]);

  const pause = useCallback(() => {
    pausedRef.current = true;
    setState((prev) => ({ ...prev, playing: false }));
  }, []);

  const stop = useCallback(() => {
    pausedRef.current = false;
    setState({ playing: false, elapsedInScene: 0 });
  }, []);

  // Tick forward while playing. On scene boundary, advance `activeId` to
  // the next selected scene. We drive `activeId` (not a local index) so
  // the whole UI follows.
  useEffect(() => {
    if (!state.playing) {
      clearTimer();
      return;
    }
    const tickMs = 200;
    timerRef.current = window.setInterval(() => {
      const selected = selectedRef.current;
      const idx = selected.findIndex((s) => s.id === activeIdRef.current);
      const scene = selected[idx < 0 ? 0 : idx];
      if (!scene) {
        setState({ playing: false, elapsedInScene: 0 });
        return;
      }
      setState((prev) => {
        const nextElapsed = prev.elapsedInScene + tickMs / 1000;
        if (nextElapsed >= scene.duration) {
          const nextIdx = (idx < 0 ? 0 : idx) + 1;
          if (nextIdx >= selected.length) {
            return { playing: false, elapsedInScene: 0 };
          }
          // Advance to the next scene. This updates the parent's
          // activeId, which re-derives currentSceneIndex on re-render.
          onActiveChangeRef.current(selected[nextIdx].id);
          return { playing: true, elapsedInScene: 0 };
        }
        return { ...prev, elapsedInScene: nextElapsed };
      });
    }, tickMs);
    return clearTimer;
  }, [state.playing, clearTimer]);

  // Stop playback when there is nothing selected, and reset the tick when
  // the active scene changes so we don't carry over stale elapsed time.
  useEffect(() => {
    if (selectedScenes.length === 0) {
      setState({ playing: false, elapsedInScene: 0 });
    }
  }, [selectedScenes.length]);

  // biome-ignore lint/correctness/useExhaustiveDependencies(activeId): activeId is an intentional trigger — the tick resets whenever the active scene changes.
  useEffect(() => {
    setState((prev) => ({ ...prev, elapsedInScene: 0 }));
  }, [activeId]);

  return {
    playing: state.playing,
    currentSceneIndex,
    elapsedInScene: state.elapsedInScene,
    totalElapsed,
    totalDuration,
    play,
    pause,
    stop,
  };
}
