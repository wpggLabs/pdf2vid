import { CheckCircle, Warning, X } from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import type { ExportProgress, ProjectWarning, TranslationWarning, WarningCode } from "../api";
import {
  cancelExport,
  getSystemStatus,
  onExportComplete,
  onExportError,
  onExportProgress,
} from "../backend";

interface DoneState {
  youtubePath: string | null;
  tiktokPath: string | null;
  translationWarnings: TranslationWarning[];
  skippedPages: number[];
  untranslatedCount: number;
  warnings: ProjectWarning[];
  renderFallbackUsed: boolean;
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
      } else if (s.fontAvailable === false) {
        setSystemNote(
          "No drawtext font was found. Exports will run without on-screen narration until a font is installed.",
        );
      }
    });
  }, []);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    (async () => {
      unlisteners.push(await onExportProgress((p) => p.jobId === jobId && setProgress(p)));
      unlisteners.push(
        await onExportComplete(
          (c) =>
            c.jobId === jobId &&
            setDone({
              youtubePath: c.youtubePath,
              tiktokPath: c.tiktokPath,
              translationWarnings: c.translationWarnings ?? [],
              skippedPages: c.skippedPages ?? [],
              untranslatedCount: c.untranslatedCount ?? 0,
              warnings: c.warnings ?? [],
              renderFallbackUsed: c.renderFallbackUsed ?? false,
            }),
        ),
      );
      unlisteners.push(await onExportError((e) => e.jobId === jobId && setError(e.message)));
    })();
    return () => {
      for (const fn of unlisteners) fn();
    };
  }, [jobId]);

  async function handleCancel() {
    try {
      await cancelExport();
    } catch (e) {
      setError(`Cancel failed: ${e}`);
    }
  }

  // Group the typed warnings by code so the UI can render each kind
  // with its own block. We keep this on the component (not a hook)
  // because the order and grouping is presentation-specific.
  const groupedWarnings = useMemo(() => groupWarnings(done?.warnings ?? []), [done?.warnings]);

  const totalWarnings =
    (done?.translationWarnings.length ?? 0) +
    (done?.skippedPages.length ?? 0) +
    (done?.warnings.filter((w) => w.severity !== "info").length ?? 0);
  const reviewCount = done?.untranslatedCount ?? 0;

  return (
    <div className="modal-backdrop">
      <section className="modal" role="dialog" aria-modal="true" aria-label="Export in progress">
        <header>
          <h2>{done ? "Export complete" : error ? "Export failed" : "Exporting…"}</h2>
          {done || error ? (
            <button type="button" className="icon-button" onClick={onClose} aria-label="Close">
              <X size={18} />
            </button>
          ) : null}
        </header>

        {systemNote && !progress && <p className="modal-warning">{systemNote}</p>}

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
              <div className="progress-modal-bar-fill" style={{ width: `${progress.percent}%` }} />
            </div>
            <span className="progress-modal-percent">{progress.percent}%</span>
            <button type="button" className="export-secondary" onClick={handleCancel}>
              Cancel
            </button>
          </div>
        )}

        {done && (
          <div className="progress-done">
            <div className="progress-summary-header">
              <CheckCircle size={28} weight="fill" />
              <div>
                <strong>Export complete</strong>
                <span>
                  {totalWarnings === 0 && reviewCount === 0
                    ? "Everything looks clean."
                    : `${totalWarnings} warning${totalWarnings === 1 ? "" : "s"}, ${reviewCount} untranslated scene${reviewCount === 1 ? "" : "s"}.`}
                </span>
              </div>
            </div>

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

            {done.renderFallbackUsed && (
              <div className="progress-warning-block">
                <Warning size={16} weight="fill" />
                <div>
                  <strong>Render fell back to text-less drawtext</strong>
                  <span>
                    No font was available, so the exported videos do not display on-screen
                    narration. Install a TrueType font and re-export to restore captions.
                  </span>
                </div>
              </div>
            )}

            {groupedWarnings.missingFont.length > 0 && (
              <div className="progress-warning-block">
                <Warning size={16} weight="fill" />
                <div>
                  <strong>Missing drawtext font</strong>
                  <span>{groupedWarnings.missingFont[0].message}</span>
                  {groupedWarnings.missingFont[0].suggestedFix && (
                    <code className="progress-warning-fix">
                      {groupedWarnings.missingFont[0].suggestedFix}
                    </code>
                  )}
                </div>
              </div>
            )}

            {done.skippedPages.length > 0 && (
              <div className="progress-warning-block">
                <Warning size={16} weight="fill" />
                <div>
                  <strong>
                    {done.skippedPages.length} page{done.skippedPages.length === 1 ? "" : "s"}{" "}
                    skipped
                  </strong>
                  <span>
                    Imported PDF had no selectable text on these pages:{" "}
                    {done.skippedPages.slice(0, 8).join(", ")}
                    {done.skippedPages.length > 8 && `, +${done.skippedPages.length - 8} more`}
                  </span>
                </div>
              </div>
            )}

            {done.translationWarnings.length > 0 && (
              <div className="progress-warnings">
                <p>
                  <Warning size={16} />
                  <strong>
                    {done.translationWarnings.length} scene
                    {done.translationWarnings.length === 1 ? "" : "s"} used the source script
                    because translation wasn't available.
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
                  Switch translation provider in the inspector to OpenAI or Google Cloud for actual
                  translation.
                </p>
              </div>
            )}

            {groupedWarnings.unsupportedProvider.length > 0 && (
              <div className="progress-warnings">
                <p>
                  <Warning size={16} />
                  <strong>Unsupported provider selected</strong>
                </p>
                <ul>
                  {groupedWarnings.unsupportedProvider.slice(0, 5).map((w, i) => (
                    <li key={i}>{w.message}</li>
                  ))}
                </ul>
                {groupedWarnings.unsupportedProvider[0]?.suggestedFix && (
                  <p className="progress-warning-hint">
                    {groupedWarnings.unsupportedProvider[0].suggestedFix}
                  </p>
                )}
              </div>
            )}

            <button type="button" className="export-primary" onClick={onClose}>
              Done
            </button>
          </div>
        )}

        {error && (
          <div className="progress-error">
            <p>{error}</p>
            <button type="button" className="export-secondary" onClick={onClose}>
              Close
            </button>
          </div>
        )}
      </section>
    </div>
  );
}

interface GroupedWarnings {
  missingFont: ProjectWarning[];
  renderFallback: ProjectWarning[];
  unsupportedProvider: ProjectWarning[];
  voiceSynthesisFailed: ProjectWarning[];
  untranslatedScene: ProjectWarning[];
  missingDependency: ProjectWarning[];
  other: ProjectWarning[];
}

function groupWarnings(warnings: ProjectWarning[]): GroupedWarnings {
  const groups: GroupedWarnings = {
    missingFont: [],
    renderFallback: [],
    unsupportedProvider: [],
    voiceSynthesisFailed: [],
    untranslatedScene: [],
    missingDependency: [],
    other: [],
  };
  for (const w of warnings) {
    const bucket = bucketFor(w.code);
    if (bucket) groups[bucket].push(w);
    else groups.other.push(w);
  }
  return groups;
}

function bucketFor(code: WarningCode): keyof GroupedWarnings | null {
  switch (code) {
    case "missingFont":
      return "missingFont";
    case "renderFallback":
      return "renderFallback";
    case "unsupportedProvider":
      return "unsupportedProvider";
    case "voiceSynthesisFailed":
      return "voiceSynthesisFailed";
    case "untranslatedScene":
      return "untranslatedScene";
    case "missingDependency":
      return "missingDependency";
    default:
      return null;
  }
}
