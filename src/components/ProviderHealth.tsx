import { Check, Copy, Warning, X } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import type { DependencyStatus, InstallHint, LocalDep } from "../backend";
import { dependencyStatus, localDeps } from "../backend";

interface Props {
  onOpenModels?: () => void;
}

export function ProviderHealth({ onOpenModels }: Props) {
  const [status, setStatus] = useState<DependencyStatus | null>(null);
  const [optional, setOptional] = useState<LocalDep[] | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    dependencyStatus()
      .then(setStatus)
      .catch(() => undefined);
    localDeps()
      .then(setOptional)
      .catch(() => undefined);
  }, []);

  async function copy(command: string, id: string) {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(id);
      setTimeout(() => setCopied((c) => (c === id ? null : c)), 1500);
    } catch {
      // Clipboard can be unavailable; the command is still visible to copy manually.
    }
  }

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
          ? `Python found, but edge-tts is not installed`
          : "Python not found",
      hint: status.installHints.find((h: InstallHint) => h.tool === "python"),
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
                <button
                  type="button"
                  className="provider-health-cmd"
                  onClick={() => copy(row.hint?.command ?? "", row.tool)}
                  title="Copy install command"
                >
                  <code>{row.hint.command}</code>
                  {copied === row.tool ? <Check size={12} weight="bold" /> : <Copy size={12} />}
                </button>
              )}
            </div>
          </li>
        ))}
      </ul>

      {optional && optional.length > 0 && (
        <div className="provider-health-optional">
          <div className="provider-health-head">
            <strong>Optional local models</strong>
          </div>
          <p className="provider-health-hint-text">
            Install any of these to unlock offline translation and higher-quality voices. Each is a
            one-time <code>pip install</code>; the app downloads the model on first use.
          </p>
          <ul>
            {optional.map((dep) => (
              <li key={dep.id} className={dep.installed ? "ok" : "warn"}>
                <span className="provider-health-icon">
                  {dep.installed ? (
                    <Check size={14} weight="bold" />
                  ) : (
                    <X size={14} weight="bold" />
                  )}
                </span>
                <div className="provider-health-row">
                  <strong>{dep.label}</strong>
                  <span className="provider-health-detail">
                    {dep.installed ? "Installed" : dep.purpose}
                  </span>
                  {!dep.installed && (
                    <button
                      type="button"
                      className="provider-health-cmd"
                      onClick={() => copy(dep.command, dep.id)}
                      title="Copy install command"
                    >
                      <code>{dep.command}</code>
                      {copied === dep.id ? <Check size={12} weight="bold" /> : <Copy size={12} />}
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {!status.ffmpeg && (
        <div className="provider-health-warning">
          <Warning size={16} weight="fill" />
          <span>
            FFmpeg is not installed. Install it to enable export. Until then the app can import and
            preview, but cannot produce a video file.
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
