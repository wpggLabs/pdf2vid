import { X, Export, Gear } from "@phosphor-icons/react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { Project } from "../types";
import { useState } from "react";

interface Props {
  onClose: () => void;
  onStart: (jobId: string, outputDir: string) => void;
  onOpenSettings: () => void;
  project: Project;
}

export function ExportModal({ onClose, onStart, onOpenSettings, project }: Props) {
  const [busy, setBusy] = useState(false);
  const totalDuration = project.scenes
    .filter((scene) => scene.selected)
    .reduce((sum, scene) => sum + scene.duration, 0);
  const selectedCount = project.scenes.filter((scene) => scene.selected).length;

  async function handleStart() {
    setBusy(true);
    try {
      const dir = await saveDialog({
        title: "Choose output folder",
        defaultPath: `${project.name}-video`,
        filters: [{ name: "Video folder", extensions: [] }],
      });
      if (!dir || typeof dir !== "string") {
        setBusy(false);
        return;
      }
      const jobId = crypto.randomUUID();
      onStart(jobId, dir);
      onClose();
    } catch (e) {
      alert(`Could not start export: ${e}`);
      setBusy(false);
    }
  }

  const hasCloudKey = (provider: string) => provider === "edge" || provider === "marian" || provider === "piper" || provider === "pages";

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <section
        className="modal"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Export video"
      >
        <header>
          <h2>Export video</h2>
          <button className="icon-button" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </header>
        <div className="export-summary">
          <Export size={36} />
          <strong>{project.name}</strong>
          <span>
            {formatTime(totalDuration)} · {selectedCount} scene{selectedCount === 1 ? "" : "s"}
          </span>
        </div>
        <label className="check-row">
          <input type="checkbox" checked={project.outputYouTube} readOnly />
          <div>
            <strong>YouTube landscape</strong>
            <span>1920×1080 · H.264</span>
          </div>
        </label>
        <label className="check-row">
          <input type="checkbox" checked={project.outputTikTok} readOnly />
          <div>
            <strong>TikTok portrait</strong>
            <span>1080×1920 · H.264</span>
          </div>
        </label>

        {!hasCloudKey(project.translationProvider) || !hasCloudKey(project.voiceProvider) ? (
          <div className="export-warning">
            <Gear size={18} />
            <span>Some providers need configuration before export.</span>
            <button className="link" onClick={onOpenSettings}>
              Open settings
            </button>
          </div>
        ) : null}

        <button
          className="export-primary"
          onClick={handleStart}
          disabled={busy || selectedCount === 0}
        >
          <Export size={18} /> {busy ? "Starting…" : "Start export"}
        </button>
      </section>
    </div>
  );
}

function formatTime(value: number) {
  const minutes = Math.floor(value / 60);
  return `${String(minutes).padStart(2, "0")}:${String(value % 60).padStart(2, "0")}`;
}