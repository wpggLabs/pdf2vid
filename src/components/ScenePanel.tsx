import { Check, FilePdf, Plus } from "@phosphor-icons/react";
import { useRef } from "react";
import { seconds } from "../lib/format";
import type { ImportProgress, ImportSummary } from "../state/useProjectState";
import type { Scene } from "../types";

interface ScenePanelProps {
  sourceName: string;
  scenes: Scene[];
  activeId: string | null;
  totalDuration: number;
  importProgress: ImportProgress | null;
  importSummary: ImportSummary;
  onImportFile: (file: File) => void;
  onPickPdf: () => void;
  onSelectScene: (id: string) => void;
  onToggleScene: (id: string, selected: boolean) => void;
}

export function ScenePanel({
  sourceName,
  scenes,
  activeId,
  totalDuration,
  importProgress,
  importSummary,
  onImportFile,
  onPickPdf,
  onSelectScene,
  onToggleScene,
}: ScenePanelProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  return (
    <aside className="scene-panel">
      <div className="panel-heading">
        <div>
          <span>PROJECT</span>
          <strong>{sourceName}</strong>
        </div>
        <button
          type="button"
          className="icon-button"
          onClick={() => inputRef.current?.click()}
          aria-label="Import PDF"
        >
          <Plus size={18} />
        </button>
      </div>
      <input
        ref={inputRef}
        type="file"
        accept="application/pdf"
        hidden
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file) onImportFile(file);
        }}
      />
      <button type="button" className="import-button" onClick={onPickPdf}>
        <FilePdf size={20} />
        Import PDF
      </button>
      {importProgress && (
        <div className="import-progress">
          <span>
            Reading page {importProgress.page} of {importProgress.total}
          </span>
          <div className="import-progress-bar">
            <div
              className="import-progress-bar-fill"
              style={{ width: `${(importProgress.page / importProgress.total) * 100}%` }}
            />
          </div>
        </div>
      )}
      {importSummary.status && !importProgress && (
        <div className="import-summary" data-testid="import-summary">
          <strong>
            {importSummary.imported} page{importSummary.imported === 1 ? "" : "s"} imported
          </strong>
          {importSummary.skipped.length > 0 && (
            <span>
              {importSummary.skipped.length} skipped (no text):{" "}
              {importSummary.skipped.slice(0, 3).join(", ")}
              {importSummary.skipped.length > 3 && `, +${importSummary.skipped.length - 3} more`}
            </span>
          )}
          {importSummary.needsOcr && (
            <span className="import-summary-warn">OCR required — no selectable text.</span>
          )}
          {importSummary.translationNeeded && importSummary.imported > 0 && (
            <span className="import-summary-hint">
              Review translation provider in the inspector.
            </span>
          )}
        </div>
      )}
      <div className="scene-label">
        <span>SCENES</span>
        <span>
          {scenes.filter((scene) => scene.selected).length} / {scenes.length}
        </span>
      </div>
      <div className="scene-list">
        {scenes.map((scene, index) => (
          <article
            key={scene.id}
            className={`scene-row ${scene.id === activeId ? "selected" : ""}`}
            onClick={() => onSelectScene(scene.id)}
          >
            <button
              type="button"
              className={`select-box ${scene.selected ? "checked" : ""}`}
              onClick={(event) => {
                event.stopPropagation();
                onToggleScene(scene.id, !scene.selected);
              }}
              aria-label={`Select page ${scene.page}`}
            >
              {scene.selected && <Check size={12} weight="bold" />}
            </button>
            <div className="thumb">
              {scene.thumbnail ? <img src={scene.thumbnail} alt="" /> : <FilePdf size={24} />}
              <b>{index + 1}</b>
            </div>
            <div className="scene-meta">
              <strong>{scene.title}</strong>
              <span>Page {scene.page}</span>
              <time>{seconds(scene.duration)}</time>
            </div>
          </article>
        ))}
      </div>
      <footer>
        <span>Total duration</span>
        <time>{seconds(totalDuration)}</time>
      </footer>
    </aside>
  );
}
