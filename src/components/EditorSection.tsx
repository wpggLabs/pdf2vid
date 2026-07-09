import {
  ArrowsOut,
  FilePdf,
  Pause,
  Play,
  SkipBack,
  SkipForward,
  SpeakerHigh,
  Trash,
  Waveform,
} from "@phosphor-icons/react";
import { seconds } from "../lib/format";
import type { Scene } from "../types";

interface EditorSectionProps {
  scenes: Scene[];
  active: Scene;
  activeId: string | null;
  aspect: "youtube" | "tiktok";
  timelineTab: "timeline" | "subtitles";
  playing: boolean;
  totalElapsed: number;
  totalDuration: number;
  previewCaption: string | null;
  onAspectChange: (aspect: "youtube" | "tiktok") => void;
  onTimelineTab: (tab: "timeline" | "subtitles") => void;
  onToggleFullscreen: () => void;
  onSelectScene: (id: string) => void;
  onSkipBack: () => void;
  onSkipForward: () => void;
  onTogglePlay: () => void;
  onScriptChange: (script: string) => void;
  onDeleteScene: () => void;
}

export function EditorSection({
  scenes,
  active,
  activeId,
  aspect,
  timelineTab,
  playing,
  totalElapsed,
  totalDuration,
  previewCaption,
  onAspectChange,
  onTimelineTab,
  onToggleFullscreen,
  onSelectScene,
  onSkipBack,
  onSkipForward,
  onTogglePlay,
  onScriptChange,
  onDeleteScene,
}: EditorSectionProps) {
  const selectedScenes = scenes.filter((s) => s.selected);

  return (
    <section className="editor">
      <div className="preview-toolbar">
        <select
          value={aspect}
          onChange={(event) => onAspectChange(event.target.value as "youtube" | "tiktok")}
        >
          <option value="youtube">YouTube · 1920×1080</option>
          <option value="tiktok">TikTok · 1080×1920</option>
        </select>
        <button
          type="button"
          className="icon-button"
          onClick={onToggleFullscreen}
          aria-label="Fullscreen"
        >
          <ArrowsOut size={18} />
        </button>
      </div>
      <div className={`preview-stage ${aspect}`}>
        <div className="paper-preview">
          {active.thumbnail ? (
            <img src={active.thumbnail} alt={`PDF page ${active.page}`} />
          ) : (
            <div className="empty-preview">
              <FilePdf size={54} />
              <strong>Import a PDF to begin</strong>
            </div>
          )}
          {previewCaption && <p>{previewCaption}</p>}
        </div>
      </div>
      <div className="transport">
        <span>{seconds(totalElapsed)}</span>
        <div className="transport-actions">
          <button type="button" onClick={onSkipBack} aria-label="Previous scene">
            <SkipBack weight="fill" />
          </button>
          <button
            type="button"
            className="play"
            onClick={onTogglePlay}
            aria-label={playing ? "Pause" : "Play"}
          >
            {playing ? <Pause weight="fill" /> : <Play weight="fill" />}
          </button>
          <button type="button" onClick={onSkipForward} aria-label="Next scene">
            <SkipForward weight="fill" />
          </button>
        </div>
        <span>{seconds(totalDuration)}</span>
        <SpeakerHigh size={18} />
      </div>
      <div className="timeline-tabs">
        <button
          type="button"
          className={timelineTab === "timeline" ? "active" : ""}
          onClick={() => onTimelineTab("timeline")}
        >
          TIMELINE
        </button>
        <button
          type="button"
          className={timelineTab === "subtitles" ? "active" : ""}
          onClick={() => onTimelineTab("subtitles")}
        >
          SUBTITLES
        </button>
      </div>
      {timelineTab === "timeline" ? (
        <div className="timeline">
          <div className="time-ruler">
            <span>0:00</span>
            <span>{seconds(Math.round(totalDuration / 2))}</span>
            <span>{seconds(totalDuration)}</span>
          </div>
          <div className="clip-track">
            {scenes.map((scene, index) => (
              <button
                type="button"
                key={scene.id}
                className={scene.id === activeId ? "active" : ""}
                style={{ flex: scene.duration }}
                onClick={() => onSelectScene(scene.id)}
              >
                <b>{index + 1}</b>
                <span>{seconds(scene.duration)}</span>
              </button>
            ))}
          </div>
          <div className="audio-track">
            <Waveform size={17} />
            <div>
              {Array.from({ length: 58 }, (_, index) => (
                <i key={index} style={{ height: `${18 + ((index * 13) % 30)}%` }} />
              ))}
            </div>
          </div>
          <div className="subtitle-track">
            <span>CC</span>
            {scenes.map((scene) => (
              <button
                type="button"
                key={scene.id}
                style={{ flex: scene.duration }}
                onClick={() => onSelectScene(scene.id)}
              >
                {scene.script}
              </button>
            ))}
          </div>
        </div>
      ) : (
        <div className="subtitles-view">
          <div className="subtitle-list">
            {selectedScenes.map((scene, i) => (
              <article key={scene.id} className="subtitle-row">
                <span className="subtitle-index">{i + 1}</span>
                <span className="subtitle-time">
                  {seconds(selectedScenes.slice(0, i).reduce((sum, s) => sum + s.duration, 0))}
                </span>
                <p>{scene.script}</p>
              </article>
            ))}
            {selectedScenes.length === 0 && (
              <p className="subtitle-empty">Select scenes to populate subtitles.</p>
            )}
          </div>
        </div>
      )}
      <div className="script-editor">
        <div>
          <span>SCENE SCRIPT</span>
          <span>{active.script.length} / 50000</span>
        </div>
        <textarea
          value={active.script}
          maxLength={50000}
          onChange={(event) => onScriptChange(event.target.value)}
        />
        <button type="button" className="delete" onClick={onDeleteScene}>
          <Trash size={16} />
          Delete scene
        </button>
      </div>
    </section>
  );
}
