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

  const preview = useCallback(
    async (provider: string, voice: string, text: string) => {
      if (!text.trim()) {
        setState({ loading: false, error: "Nothing to preview", audioUrl: null });
        return;
      }
      setState({ loading: true, error: null, audioUrl: null });
      try {
        const url = await backendPreviewVoice(provider, voice, text);
        if (audioRef.current) {
          audioRef.current.src = url;
          await audioRef.current.play().catch(() => undefined);
        }
        setState({ loading: false, error: null, audioUrl: url });
      } catch (e) {
        setState({
          loading: false,
          error: e instanceof Error ? e.message : String(e),
          audioUrl: null,
        });
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