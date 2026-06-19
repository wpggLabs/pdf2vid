import { useEffect, useRef, useState } from "react";
import { X, Play, Pause, SkipBack, SkipForward } from "@phosphor-icons/react";
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
 * Scene-keyed request IDs prevent stale `previewVoice` responses from
 * overwriting the audio for the currently visible scene when the user
 * clicks Skip quickly.
 */
export function PreviewModal({ onClose, scene, voiceProvider, voice, scenes, onSceneChange }: Props) {
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const requestSeq = useRef(0);

  // Generate audio for the current scene. Bumping the sequence number on
  // every dependency change ensures that an in-flight request from an older
  // scene can't clobber the result for the newer scene.
  useEffect(() => {
    const mySeq = ++requestSeq.current;
    setLoading(true);
    setError(null);
    setAudioUrl(null);
    previewVoice(voiceProvider, voice, scene.script)
      .then((url) => {
        if (mySeq !== requestSeq.current) return; // stale
        setAudioUrl(url);
        setLoading(false);
      })
      .catch((e) => {
        if (mySeq !== requestSeq.current) return; // stale
        setError(String(e));
        setLoading(false);
      });
  }, [scene.id, scene.script, voiceProvider, voice]);

  // Bind audio element events.
  useEffect(() => {
    const el = audioRef.current;
    if (!el) return;
    const onPlay = () => setPlaying(true);
    const onPause = () => setPlaying(false);
    const onEnded = () => setPlaying(false);
    el.addEventListener("play", onPlay);
    el.addEventListener("pause", onPause);
    el.addEventListener("ended", onEnded);
    return () => {
      el.removeEventListener("play", onPlay);
      el.removeEventListener("pause", onPause);
      el.removeEventListener("ended", onEnded);
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
    if (playing) el.pause();
    else el.play().catch(() => undefined);
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
          <button className="icon-button" onClick={onClose} aria-label="Close">
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
          <button onClick={goPrev} aria-label="Previous scene">
            <SkipBack weight="fill" />
          </button>
          <button
            className="play"
            onClick={togglePlay}
            disabled={loading || !!error || !audioUrl}
            aria-label={playing ? "Pause" : "Play"}
          >
            {loading ? "…" : playing ? <Pause weight="fill" /> : <Play weight="fill" />}
          </button>
          <button onClick={goNext} aria-label="Next scene">
            <SkipForward weight="fill" />
          </button>
        </div>

        {error && <p className="preview-error">{error}</p>}

        {audioUrl && <audio ref={audioRef} src={audioUrl} preload="auto" />}
      </section>
    </div>
  );
}