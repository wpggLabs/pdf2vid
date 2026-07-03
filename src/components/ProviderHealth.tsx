import { Check, Warning, X } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import type { DependencyStatus, InstallHint } from "../backend";
import { dependencyStatus } from "../backend";

interface Props {
  onOpenModels?: () => void;
}

export function ProviderHealth({ onOpenModels }: Props) {
  const [status, setStatus] = useState<DependencyStatus | null>(null);

  useEffect(() => {
    dependencyStatus()
      .then(setStatus)
      .catch(() => undefined);
  }, []);

  if (!status) {
    return <div className="provider-health">Checking tools…</div>;
  }

  const rows: HealthRow[] = [
    {
      tool: "FFmpeg",
      ok: status.ffmpeg,
      detail: status.ffmpegPath ?? "not found",
      hint: status.installHints.find((h: InstallHint) => h.tool === "ffmpeg"),
    },
    {
      tool: "FFprobe",
      ok: status.ffprobe,
      detail: status.ffmpeg ? "bundled with ffmpeg" : "not found",
      hint: status.installHints.find((h: InstallHint) => h.tool === "ffprobe"),
    },
    {
      tool: "edge-tts (Python)",
      ok: status.edgeTts,
      detail: status.edgeTtsVersion
        ? `installed (${status.edgeTtsVersion})`
        : status.pythonPath
          ? `Python ${status.pythonPath} found, but edge-tts is not installed`
          : "Python not found",
      hint: status.installHints.find((h: InstallHint) => h.tool === "python"),
    },
    {
      tool: "MarianMT",
      ok: false,
      detail: "Not implemented yet. Pick OpenAI or Google Cloud for translation.",
      hint: undefined,
    },
    {
      tool: "Piper",
      ok: false,
      detail: "Not implemented yet. Use edge-tts or cloud voices.",
      hint: undefined,
    },
  ];

  return (
    <div className="provider-health">
      <div className="provider-health-head">
        <strong>Runtime tools</strong>
        <span className="provider-health-platform">{status.platform}</span>
      </div>
      <ul>
        {rows.map((row) => (
          <li key={row.tool} className={row.ok ? "ok" : "warn"}>
            <span className="provider-health-icon">
              {row.ok ? <Check size={14} weight="bold" /> : <X size={14} weight="bold" />}
            </span>
            <div className="provider-health-row">
              <strong>{row.tool}</strong>
              <span className="provider-health-detail">{row.detail}</span>
              {!row.ok && row.hint && (
                <code className="provider-health-cmd">{row.hint.command}</code>
              )}
            </div>
          </li>
        ))}
      </ul>
      {!status.ffmpeg && (
        <div className="provider-health-warning">
          <Warning size={16} weight="fill" />
          <span>
            FFmpeg is not installed. Install it to enable export. Until then the app can import and
            preview, but cannot produce a video file.
          </span>
        </div>
      )}
      {status.ffmpeg && !status.edgeTts && (
        <div className="provider-health-warning">
          <Warning size={16} weight="fill" />
          <span>
            edge-tts is not installed. Voice synthesis will fall back to StreamElements / Google
            TTS. Install Python + edge-tts for the best free voice quality.
          </span>
        </div>
      )}
      {onOpenModels && (
        <button type="button" className="link" onClick={onOpenModels}>
          Manage local models
        </button>
      )}
    </div>
  );
}

interface HealthRow {
  tool: string;
  ok: boolean;
  detail: string;
  hint: InstallHint | undefined;
}
