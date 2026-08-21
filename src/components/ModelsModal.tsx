import { Check, Download, Stop, Trash, X } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import type { ModelDownloadProgress, ModelInfo } from "../api";
import {
  cancelModelDownload,
  deleteModel,
  downloadModel,
  getModels,
  onModelComplete,
  onModelProgress,
} from "../backend";

interface Props {
  onClose: () => void;
}

export function ModelsModal({ onClose }: Props) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [progress, setProgress] = useState<Record<string, ModelDownloadProgress>>({});
  const [error, setError] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);

  useEffect(() => {
    getModels()
      .then(setModels)
      .catch((e) => setError(String(e)));
    const unlistenProgress = onModelProgress((p) =>
      setProgress((prev) => ({ ...prev, [p.modelId]: p })),
    );
    const unlistenComplete = onModelComplete(({ modelId, success }) => {
      if (success) {
        getModels()
          .then(setModels)
          .catch(() => undefined);
      }
      setProgress((prev) => {
        const next = { ...prev };
        delete next[modelId];
        return next;
      });
      setCancelling(false);
    });
    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, []);

  function handleDownload(model: ModelInfo) {
    if (model.requiresAccept) {
      const ok = confirm(`This model is licensed ${model.license}. Accept and download?`);
      if (!ok) return;
    }
    setError(null);
    downloadModel(model.id)
      .then(() => getModels().then(setModels))
      .catch((e) => setError(String(e)));
  }

  async function handleCancel() {
    setCancelling(true);
    try {
      await cancelModelDownload();
    } catch (e) {
      setError(`Cancel failed: ${e}`);
      setCancelling(false);
    }
  }

  function handleDelete(model: ModelInfo) {
    if (!confirm(`Remove ${model.label}? You'll need to re-download to use it again.`)) return;
    deleteModel(model.id)
      .then(() => getModels().then(setModels))
      .catch((e) => setError(String(e)));
  }

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <section
        className="modal modal-wide"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Local models"
      >
        <header>
          <h2>Local models</h2>
          <button type="button" className="icon-button" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </header>

        {error && <p className="modal-error">{error}</p>}

        <p className="modal-note">
          Free local providers need model files. They're downloaded once and cached on this device.
        </p>

        {models.length === 0 && !error && (
          <p className="modal-note">
            No downloadable local models yet. Argos (translation) and edge-tts (voice) manage their
            own models separately and need no download here.
          </p>
        )}

        <div className="model-list">
          {models.map((model) => {
            const prog = progress[model.id];
            return (
              <article key={model.id} className={`model-row ${model.installed ? "installed" : ""}`}>
                <div>
                  <strong>{model.label}</strong>
                  <span>
                    {model.family} · {formatSize(model.sizeBytes)} · {model.license}
                  </span>
                </div>
                {prog ? (
                  <div className="model-progress">
                    <div className="model-progress-bar" style={{ width: `${prog.percent}%` }} />
                    <span>
                      {prog.percent}% · {formatSize(prog.downloaded)} / {formatSize(prog.total)}
                    </span>
                    <button
                      type="button"
                      className="icon-button"
                      onClick={handleCancel}
                      disabled={cancelling}
                      aria-label="Cancel download"
                    >
                      <Stop size={14} />
                    </button>
                  </div>
                ) : model.installed ? (
                  <button
                    type="button"
                    className="icon-button"
                    onClick={() => handleDelete(model)}
                    aria-label="Remove"
                  >
                    <Check size={16} /> <Trash size={14} />
                  </button>
                ) : (
                  <button
                    type="button"
                    className="export-primary compact"
                    onClick={() => handleDownload(model)}
                  >
                    <Download size={14} /> Download
                  </button>
                )}
              </article>
            );
          })}
        </div>
      </section>
    </div>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}
