import { Pause, Play, SkipBack, SkipForward, X } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { previewVoice } from "../backend";
import type { Scene } from "../types";

interface Props {
  onClose: () => void;
  scene: Scene;
  voiceProvider: string;
  voice: string;
  scenes: Scene[];
  onSceneChange: (id: string) => void;
}

/**
 * Full-screen preview modal: shows the active scene large, plays synthesized
 * audio for narration, and exposes Play/Pause/Skip transport.
 *
 * The play button is disabled until the audio element reports it can play
 * the new src (`canplay` event). When the scene changes, the audio is paused
 * and reset, and the request-sequence guard prevents stale responses from
 * clobbering the new scene's audio.
 */
export function PreviewModal({
  onClose,
  scene,
  voiceProvider,
  voice,
  scenes,
  onSceneChange,
}: Props) {
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const requestSeq = useRef(0);

  // Generate audio for the current scene. The sequence guard discards
  // stale responses from previous scenes.
  useEffect(() => {
    const mySeq = ++requestSeq.current;
    setLoading(true);
    setError(null);
    setAudioUrl(null);
    setReady(false);
    previewVoice(voiceProvider, voice, scene.script)
      .then((url) => {
        if (mySeq !== requestSeq.current) return; // stale
        setAudioUrl(url);
        // Don't flip loading=false here — wait for canplay below.
      })
      .catch((e) => {
        if (mySeq !== requestSeq.current) return;
        setError(String(e));
        setLoading(false);
      });
  }, [scene.id, scene.script, voiceProvider, voice]);

  // Wire up audio element events. We rely on `canplay` to know the new
  // src is decodable, then set ready=true so Play can be enabled.
  useEffect(() => {
    const el = audioRef.current;
    if (!el) return;
    const onCanPlay = () => {
      setReady(true);
      setLoading(false);
    };
    const onPlay = () => setPlaying(true);
    const onPause = () => setPlaying(false);
    const onEnded = () => setPlaying(false);
    const onError = () => {
      setError(`Audio failed to load`);
      setLoading(false);
      setReady(false);
    };
    el.addEventListener("canplay", onCanPlay);
    el.addEventListener("play", onPlay);
    el.addEventListener("pause", onPause);
    el.addEventListener("ended", onEnded);
    el.addEventListener("error", onError);
    // canplay may have fired before the listener attached (small race).
    if (el.readyState >= 3 /* HAVE_FUTURE_DATA */) {
      setReady(true);
      setLoading(false);
    }
    return () => {
      el.removeEventListener("canplay", onCanPlay);
      el.removeEventListener("play", onPlay);
      el.removeEventListener("pause", onPause);
      el.removeEventListener("ended", onEnded);
      el.removeEventListener("error", onError);
    };
  }, [audioUrl]);

  // Stop the audio when the scene changes so the previous narration
  // doesn't keep playing under a different image.
  useEffect(() => {
    const el = audioRef.current;
    if (el) {
      el.pause();
      el.currentTime = 0;
    }
    setPlaying(false);
  }, [scene.id]);

  const togglePlay = () => {
    const el = audioRef.current;
    if (!el) return;
    if (playing) {
      el.pause();
    } else {
      const playPromise = el.play();
      if (playPromise) {
        playPromise.catch(() => undefined);
      }
    }
  };

  const selectedScenes = scenes.filter((s) => s.selected);
  const currentIdx = selectedScenes.findIndex((s) => s.id === scene.id);
  const goPrev = () => {
    const prev = selectedScenes[Math.max(0, currentIdx - 1)];
    if (prev) onSceneChange(prev.id);
  };
  const goNext = () => {
    const next = selectedScenes[Math.min(selectedScenes.length - 1, currentIdx + 1)];
    if (next) onSceneChange(next.id);
  };

  const playDisabled = loading || !!error || !ready;

  return (
    <div className="modal-backdrop preview-backdrop" onMouseDown={onClose}>
      <section
        className="preview-modal"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Video preview"
      >
        <header>
          <h2>Preview · {scene.title}</h2>
          <button type="button" className="icon-button" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </header>

        <div className="preview-stage youtube">
          <div className="paper-preview">
            {scene.thumbnail ? (
              <img src={scene.thumbnail} alt={`PDF page ${scene.page}`} />
            ) : (
              <div className="empty-preview">
                <strong>No image</strong>
              </div>
            )}
            <p>{scene.script}</p>
          </div>
        </div>

        <div className="preview-transport">
          <button type="button" onClick={goPrev} aria-label="Previous scene">
            <SkipBack weight="fill" />
          </button>
          <button
            type="button"
            className="play"
            onClick={togglePlay}
            disabled={playDisabled}
            aria-label={playing ? "Pause" : "Play"}
          >
            {loading ? "…" : playing ? <Pause weight="fill" /> : <Play weight="fill" />}
          </button>
          <button type="button" onClick={goNext} aria-label="Next scene">
            <SkipForward weight="fill" />
          </button>
        </div>

        {loading && !error && <p className="preview-loading">Generating audio…</p>}
        {error && <p className="preview-error">{error}</p>}

        {audioUrl && <audio ref={audioRef} src={audioUrl} preload="auto" />}
      </section>
    </div>
  );
}
