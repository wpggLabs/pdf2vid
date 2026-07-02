import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Scene } from "../types";

interface PlaybackState {
  playing: boolean;
  currentSceneIndex: number;
  elapsedInScene: number;
}

/**
 * Timeline playback simulation: cycles through selected scenes using a
 * timer, advances the active scene, and exposes an audio element ref so
 * callers can attach preview-voice audio for the active scene.
 */
export function useTimelinePlayback(scenes: Scene[]) {
  const [state, setState] = useState<PlaybackState>({
    playing: false,
    currentSceneIndex: 0,
    elapsedInScene: 0,
  });
  const timerRef = useRef<number | null>(null);
  const selectedRef = useRef<Scene[]>([]);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  useEffect(() => {
    selectedRef.current = scenes.filter((s) => s.selected);
  }, [scenes]);

  useEffect(() => {
    audioRef.current = new Audio();
    return () => {
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current.src = "";
      }
    };
  }, []);

  const play = useCallback(() => {
    const selected = selectedRef.current;
    if (selected.length === 0) return;
    setState((prev) => {
      const startIdx = prev.currentSceneIndex >= selected.length ? 0 : prev.currentSceneIndex;
      return { playing: true, currentSceneIndex: startIdx, elapsedInScene: 0 };
    });
  }, []);

  const pause = useCallback(() => {
    setState((prev) => ({ ...prev, playing: false }));
    if (audioRef.current) audioRef.current.pause();
  }, []);

  const stop = useCallback(() => {
    setState({ playing: false, currentSceneIndex: 0, elapsedInScene: 0 });
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.currentTime = 0;
    }
  }, []);

  // Tick the simulation forward when playing.
  useEffect(() => {
    if (!state.playing) {
      if (timerRef.current) {
        window.clearInterval(timerRef.current);
        timerRef.current = null;
      }
      return;
    }
    const tickMs = 200;
    timerRef.current = window.setInterval(() => {
      setState((prev) => {
        const selected = selectedRef.current;
        if (selected.length === 0) return { ...prev, playing: false };
        const current = selected[prev.currentSceneIndex];
        const nextElapsed = prev.elapsedInScene + tickMs / 1000;
        if (nextElapsed >= current.duration) {
          const nextIdx = prev.currentSceneIndex + 1;
          if (nextIdx >= selected.length) {
            return { playing: false, currentSceneIndex: 0, elapsedInScene: 0 };
          }
          return {
            playing: true,
            currentSceneIndex: nextIdx,
            elapsedInScene: 0,
          };
        }
        return { ...prev, elapsedInScene: nextElapsed };
      });
    }, tickMs);
    return () => {
      if (timerRef.current) {
        window.clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [state.playing]);

  const selectedScenes = useMemo(() => scenes.filter((s) => s.selected), [scenes]);
  const totalDuration = selectedScenes.reduce((sum, scene) => sum + scene.duration, 0);
  const currentElapsed =
    selectedScenes
      .slice(0, state.currentSceneIndex)
      .reduce((sum, scene) => sum + scene.duration, 0) + state.elapsedInScene;

  // Keep the ref in sync for the tick effect.
  useEffect(() => {
    selectedRef.current = selectedScenes;
  }, [selectedScenes]);

  return {
    playing: state.playing,
    currentSceneIndex: state.currentSceneIndex,
    elapsedInScene: state.elapsedInScene,
    totalElapsed: currentElapsed,
    totalDuration,
    audioRef,
    play,
    pause,
    stop,
  };
}
