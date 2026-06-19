import { X, Warning } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { cancelExport, getSystemStatus, onExportComplete, onExportError, onExportProgress } from "../backend";
import type { ExportProgress, TranslationWarning } from "../api";

interface DoneState {
  youtubePath: string | null;
  tiktokPath: string | null;
  translationWarnings: TranslationWarning[];
}

interface Props {
  jobId: string;
  onClose: () => void;
}

export function ProgressModal({ jobId, onClose }: Props) {
  const [progress, setProgress] = useState<ExportProgress | null>(null);
  const [done, setDone] = useState<DoneState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [systemNote, setSystemNote] = useState<string | null>(null);

  useEffect(() => {
    getSystemStatus().then((s) => {
      if (!s.ffmpeg && !s.ffmpegSidecarReady) {
        setSystemNote(
          "FFmpeg is not detected. Install FFmpeg or place the bundled sidecar next to the app for rendering to work.",
        );
      }
    });
  }, []);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    (async () => {
      unlisteners.push(await onExportProgress((p) => p.jobId === jobId && setProgress(p)));
      unlisteners.push(
        await onExportComplete((c) =>
          c.jobId === jobId &&
          setDone({
            youtubePath: c.youtubePath,
            tiktokPath: c.tiktokPath,
            translationWarnings: c.translationWarnings ?? [],
          }),
        ),
      );
      unlisteners.push(await onExportError((e) => e.jobId === jobId && setError(e.message)));
    })();
    return () => unlisteners.forEach((fn) => fn());
  }, [jobId]);

  async function handleCancel() {
    try {
      await cancelExport();
    } catch (e) {
      setError(`Cancel failed: ${e}`);
    }
  }

  return (
    <div className="modal-backdrop">
      <section className="modal" role="dialog" aria-modal="true" aria-label="Export in progress">
        <header>
          <h2>{done ? "Export complete" : error ? "Export failed" : "Exporting…"}</h2>
          {done || error ? (
            <button className="icon-button" onClick={onClose} aria-label="Close">
              <X size={18} />
            </button>
          ) : null}
        </header>

        {systemNote && !progress && (
          <p className="modal-warning">{systemNote}</p>
        )}

        {!done && !error && progress && (
          <div className="progress-modal">
            <div className="progress-modal-stage">
              <strong>{progress.stage}</strong>
              <span>
                {progress.message}
                {progress.current && progress.total
                  ? ` · ${progress.current} / ${progress.total}`
                  : ""}
              </span>
            </div>
            <div className="progress-modal-bar">
              <div
                className="progress-modal-bar-fill"
                style={{ width: `${progress.percent}%` }}
              />
            </div>
            <span className="progress-modal-percent">{progress.percent}%</span>
            <button className="export-secondary" onClick={handleCancel}>
              Cancel
            </button>
          </div>
        )}

        {done && (
          <div className="progress-done">
            {done.youtubePath && (
              <p>
                <strong>YouTube:</strong> <code>{done.youtubePath}</code>
              </p>
            )}
            {done.tiktokPath && (
              <p>
                <strong>TikTok:</strong> <code>{done.tiktokPath}</code>
              </p>
            )}
            {done.translationWarnings.length > 0 && (
              <div className="progress-warnings">
                <p>
                  <Warning size={16} />
                  <strong>
                    {done.translationWarnings.length} scene
                    {done.translationWarnings.length === 1 ? "" : "s"} used the source
                    script because translation wasn't available.
                  </strong>
                </p>
                <ul>
                  {done.translationWarnings.slice(0, 5).map((w) => (
                    <li key={w.sceneId}>
                      Page {w.page}: {w.provider} not implemented
                    </li>
                  ))}
                  {done.translationWarnings.length > 5 && (
                    <li>...and {done.translationWarnings.length - 5} more</li>
                  )}
                </ul>
                <p className="progress-warning-hint">
                  Switch translation provider in the inspector to OpenAI or Google
                  Cloud for actual translation.
                </p>
              </div>
            )}
            <button className="export-primary" onClick={onClose}>
              Done
            </button>
          </div>
        )}

        {error && (
          <div className="progress-error">
            <p>{error}</p>
            <button className="export-secondary" onClick={onClose}>
              Close
            </button>
          </div>
        )}
      </section>
    </div>
  );
}