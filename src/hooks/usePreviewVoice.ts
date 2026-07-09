import { useCallback, useEffect, useRef, useState } from "react";
import { previewVoice as backendPreviewVoice } from "../backend";

interface PreviewState {
  loading: boolean;
  error: string | null;
  audioUrl: string | null;
}

export function usePreviewVoice() {
  const [state, setState] = useState<PreviewState>({
    loading: false,
    error: null,
    audioUrl: null,
  });
  const audioRef = useRef<HTMLAudioElement | null>(null);

  useEffect(() => {
    audioRef.current = new Audio();
    return () => {
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current.src = "";
      }
    };
  }, []);

  /**
   * Synthesize and play the narration. Resolves with the audio's real
   * duration in seconds (or null if unknown) so callers can align scene
   * timing with the actual voice length — the same thing the exporter
   * does with ffprobe on the backend.
   */
  const preview = useCallback(
    async (provider: string, voice: string, text: string, speed = 100): Promise<number | null> => {
      if (!text.trim()) {
        setState({ loading: false, error: "Nothing to preview", audioUrl: null });
        return null;
      }
      setState({ loading: true, error: null, audioUrl: null });
      try {
        const url = await backendPreviewVoice(provider, voice, text, speed);
        let duration: number | null = null;
        if (audioRef.current) {
          audioRef.current.src = url;
          await audioRef.current.play().catch(() => undefined);
          const d = audioRef.current.duration;
          duration = Number.isFinite(d) && d > 0 ? d : null;
        }
        setState({ loading: false, error: null, audioUrl: url });
        return duration;
      } catch (e) {
        setState({
          loading: false,
          error: e instanceof Error ? e.message : String(e),
          audioUrl: null,
        });
        return null;
      }
    },
    [],
  );

  const stop = useCallback(() => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.currentTime = 0;
    }
  }, []);

  return { ...state, preview, stop };
}
