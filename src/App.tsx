import {
  ArrowsOut,
  Check,
  Export,
  FilePdf,
  Gear,
  List,
  Pause,
  Play,
  Plus,
  SkipBack,
  SkipForward,
  SpeakerHigh,
  Trash,
  Warning,
  Waveform,
} from "@phosphor-icons/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { ProviderList } from "./api";
import { startExport as backendExport, getProviderList, getSystemStatus } from "./backend";
import { ExportModal } from "./components/ExportModal";
import { ModelsModal } from "./components/ModelsModal";
import { PreviewModal } from "./components/PreviewModal";
import { ProgressModal } from "./components/ProgressModal";
import { ProviderField } from "./components/ProviderField";
import { ProviderHealth } from "./components/ProviderHealth";
import { SettingsModal } from "./components/SettingsModal";
import { usePreviewVoice } from "./hooks/usePreviewVoice";
import { useTimelinePlayback } from "./hooks/useTimelinePlayback";
import { useTranslationModelPrompt } from "./hooks/useTranslationModelPrompt";
import { voiceOptionsFor } from "./lib/voiceOptions";
import { useProjectState } from "./state/useProjectState";
import { useWorkspaceUi } from "./state/useWorkspaceUi";
import type { ProviderOption, SystemStatus } from "./types";

function seconds(value: number) {
  const minutes = Math.floor(value / 60);
  return `${String(minutes).padStart(2, "0")}:${String(value % 60).padStart(2, "0")}`;
}

function providerById(options: ProviderOption[], id: string) {
  return options.find((option) => option.id === id) ?? options[0];
}

function App() {
  const proj = useProjectState();
  const {
    project,
    setProject,
    activeId,
    setActiveId,
    active,
    importProgress,
    status,
    setStatus,
    importSummary,
    importPdf,
    importPdfFromPath,
    updateScene,
    removeScene,
  } = proj;
  const ui = useWorkspaceUi();
  const [providers, setProviders] = useState<ProviderList | null>(null);
  const [system, setSystem] = useState<SystemStatus>({
    ffmpeg: false,
    ffprobe: false,
    platform: "Browser preview",
    ffmpegSidecarReady: false,
  });
  const [progressJobId, setProgressJobId] = useState<string | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);
  const previewRef = useRef<HTMLDivElement>(null);

  const duration = useMemo(
    () =>
      project.scenes
        .filter((scene) => scene.selected)
        .reduce((sum, scene) => sum + scene.duration, 0),
    [project.scenes],
  );

  const playback = useTimelinePlayback(project.scenes);
  const preview = usePreviewVoice();
  const [ttsReady, setTtsReady] = useState<boolean | null>(null);

  // Check whether Python + edge-tts is available on startup.
  useEffect(() => {
    import("./backend").then(({ checkTtsEngine }) => {
      checkTtsEngine()
        .then((status) => setTtsReady(status.pythonAvailable))
        .catch(() => setTtsReady(false));
    });
  }, []);

  const modelPrompt = useTranslationModelPrompt(
    project.translationProvider,
    project.language,
    setStatus,
  );

  // Initial load + system status (project is hydrated by useProjectState;
  // here we load providers and system info that live outside the project).
  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const [list, status] = await Promise.all([getProviderList(), getSystemStatus()]);
        if (!mounted) return;
        setProviders(list);
        setSystem(status);
        if (!status.ffmpeg && !status.ffmpegSidecarReady) {
          setStatus((prev) =>
            prev === "Ready" || prev === "Project loaded"
              ? "Ready · Install FFmpeg or bundle the sidecar to render videos"
              : prev,
          );
        }
      } catch (error) {
        if (mounted) setStatus(`Could not initialize: ${error}`);
      }
    })();
    return () => {
      mounted = false;
    };
  }, []);

  // Refresh system status on window focus.
  useEffect(() => {
    const refresh = () => {
      getSystemStatus()
        .then((s) => {
          setSystem(s);
          if (s.ffmpeg || s.ffmpegSidecarReady) {
            setStatus((prev) => (prev.includes("FFmpeg") ? "Ready" : prev));
          }
        })
        .catch(() => undefined);
    };
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, []);

  async function pickAndImportPdf() {
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (!picked || typeof picked !== "string") return;
      await importPdfFromPath(picked);
    } catch (e) {
      setStatus(`File picker failed: ${e}`);
    }
  }

  async function startExport(jobId: string, outputDir: string) {
    setProgressJobId(jobId);
    setStatus("Export queued");
    try {
      const complete = await backendExport(jobId, project, outputDir);
      const paths = [complete.youtubePath, complete.tiktokPath]
        .filter((p): p is string => Boolean(p))
        .join(", ");
      const fallbackNote = complete.renderFallbackUsed ? " · captions skipped (no font)" : "";
      const warningCount = (complete.warnings ?? []).filter((w) => w.severity !== "info").length;
      const warningNote =
        warningCount > 0 ? ` · ${warningCount} warning${warningCount === 1 ? "" : "s"}` : "";
      setStatus(`Export complete · ${paths || "saved"}${fallbackNote}${warningNote}`);
    } catch (error) {
      setStatus(`Export failed: ${error}`);
    }
  }

  // Skip back / forward = previous / next selected scene
  const selectedScenes = useMemo(() => project.scenes.filter((s) => s.selected), [project.scenes]);
  const selectedIndex = selectedScenes.findIndex((s) => s.id === activeId);

  const skipBack = useCallback(() => {
    const prev = selectedScenes[Math.max(0, selectedIndex - 1)];
    if (prev) setActiveId(prev.id);
  }, [selectedIndex, selectedScenes]);

  const skipForward = useCallback(() => {
    const next = selectedScenes[Math.min(selectedScenes.length - 1, selectedIndex + 1)];
    if (next) setActiveId(next.id);
  }, [selectedIndex, selectedScenes]);

  // Play / pause the timeline simulation
  const togglePlay = useCallback(() => {
    if (playback.playing) {
      playback.pause();
    } else {
      playback.play();
      // Kick off audio for the currently active scene so the user
      // actually hears something when they hit Play.
      preview
        .preview(project.voiceProvider, project.voice, active.script, project.voiceSpeed)
        .catch(() => undefined);
    }
  }, [playback, preview, project.voiceProvider, project.voice, active.script, project.voiceSpeed]);

  // Preview voice (active scene's script)
  const handlePreviewVoice = useCallback(() => {
    preview.preview(project.voiceProvider, project.voice, active.script, project.voiceSpeed);
  }, [preview, project.voiceProvider, project.voice, active.script, project.voiceSpeed]);

  const translationProvider = providers
    ? providerById(providers.translation, project.translationProvider)
    : null;
  const voiceProvider = providers ? providerById(providers.voice, project.voiceProvider) : null;

  const totalDuration = duration;

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span>pdf2</span>
          <strong>vid</strong>
        </div>
        <div className="project-title">
          <span>Projects</span>
          <b>/</b>
          <strong>{project.name}</strong>
        </div>
        <nav aria-label="Workspace">
          <button
            className={ui.workspaceTab === "scenes" ? "nav-active" : ""}
            onClick={() => ui.setWorkspaceTab("scenes")}
          >
            <List size={18} />
            Scenes
          </button>
          <button
            className={ui.workspaceTab === "preview" ? "nav-active" : ""}
            onClick={ui.handlePreviewTab}
          >
            <Play size={18} />
            Preview
          </button>
          <button onClick={ui.openExport}>
            <Export size={18} />
            Export
          </button>
        </nav>
        <button
          className="icon-button"
          aria-label="Settings"
          onClick={() => ui.setSettingsOpen(true)}
        >
          <Gear size={20} />
        </button>
      </header>

      <section className="workspace">
        <aside className="scene-panel">
          <div className="panel-heading">
            <div>
              <span>PROJECT</span>
              <strong>{project.sourceName}</strong>
            </div>
            <button
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
              if (file) importPdf(file);
            }}
          />
          <button className="import-button" onClick={pickAndImportPdf}>
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
                  {importSummary.skipped.length > 3 &&
                    `, +${importSummary.skipped.length - 3} more`}
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
              {project.scenes.filter((scene) => scene.selected).length} / {project.scenes.length}
            </span>
          </div>
          <div className="scene-list">
            {project.scenes.map((scene, index) => (
              <article
                key={scene.id}
                className={`scene-row ${scene.id === activeId ? "selected" : ""}`}
                onClick={() => setActiveId(scene.id)}
              >
                <button
                  className={`select-box ${scene.selected ? "checked" : ""}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    updateScene(scene.id, { selected: !scene.selected });
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
            <time>{seconds(duration)}</time>
          </footer>
        </aside>

        <section className="editor" ref={previewRef as unknown as React.RefObject<HTMLElement>}>
          <div className="preview-toolbar">
            <select
              value={ui.aspect}
              onChange={(event) => ui.setAspect(event.target.value as "youtube" | "tiktok")}
            >
              <option value="youtube">YouTube · 1920×1080</option>
              <option value="tiktok">TikTok · 1080×1920</option>
            </select>
            <button className="icon-button" onClick={ui.toggleFullscreen} aria-label="Fullscreen">
              <ArrowsOut size={18} />
            </button>
          </div>
          <div className={`preview-stage ${ui.aspect}`}>
            <div className="paper-preview">
              {active.thumbnail ? (
                <img src={active.thumbnail} alt={`PDF page ${active.page}`} />
              ) : (
                <div className="empty-preview">
                  <FilePdf size={54} />
                  <strong>Import a PDF to begin</strong>
                </div>
              )}
              <p>{active.script}</p>
            </div>
          </div>
          <div className="transport">
            <span>{seconds(playback.totalElapsed)}</span>
            <div className="transport-actions">
              <button onClick={skipBack} aria-label="Previous scene">
                <SkipBack weight="fill" />
              </button>
              <button
                className="play"
                onClick={togglePlay}
                aria-label={playback.playing ? "Pause" : "Play"}
              >
                {playback.playing ? <Pause weight="fill" /> : <Play weight="fill" />}
              </button>
              <button onClick={skipForward} aria-label="Next scene">
                <SkipForward weight="fill" />
              </button>
            </div>
            <span>{seconds(totalDuration)}</span>
            <SpeakerHigh size={18} />
          </div>
          <div className="timeline-tabs">
            <button
              className={ui.timelineTab === "timeline" ? "active" : ""}
              onClick={() => ui.setTimelineTab("timeline")}
            >
              TIMELINE
            </button>
            <button
              className={ui.timelineTab === "subtitles" ? "active" : ""}
              onClick={() => ui.setTimelineTab("subtitles")}
            >
              SUBTITLES
            </button>
          </div>
          {ui.timelineTab === "timeline" ? (
            <div className="timeline">
              <div className="time-ruler">
                <span>0:00</span>
                <span>{seconds(Math.round(duration / 2))}</span>
                <span>{seconds(duration)}</span>
              </div>
              <div className="clip-track">
                {project.scenes.map((scene, index) => (
                  <button
                    key={scene.id}
                    className={scene.id === activeId ? "active" : ""}
                    style={{ flex: scene.duration }}
                    onClick={() => setActiveId(scene.id)}
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
                {project.scenes.map((scene) => (
                  <button
                    key={scene.id}
                    style={{ flex: scene.duration }}
                    onClick={() => setActiveId(scene.id)}
                  >
                    {scene.script}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <div className="subtitles-view">
              <div className="subtitle-list">
                {project.scenes
                  .filter((s) => s.selected)
                  .map((scene, i) => (
                    <article key={scene.id} className="subtitle-row">
                      <span className="subtitle-index">{i + 1}</span>
                      <span className="subtitle-time">
                        {seconds(
                          project.scenes
                            .filter((s) => s.selected)
                            .slice(0, i)
                            .reduce((sum, s) => sum + s.duration, 0),
                        )}
                      </span>
                      <p>{scene.script}</p>
                    </article>
                  ))}
                {project.scenes.filter((s) => s.selected).length === 0 && (
                  <p className="subtitle-empty">Select scenes to populate subtitles.</p>
                )}
              </div>
            </div>
          )}
          <div className="script-editor">
            <div>
              <span>SCENE SCRIPT</span>
              <span>{active.script.length} / 5000</span>
            </div>
            <textarea
              value={active.script}
              onChange={(event) =>
                updateScene(active.id, {
                  script: event.target.value,
                  title: event.target.value.slice(0, 42),
                })
              }
            />
            <button className="delete" onClick={() => removeScene(active.id)}>
              <Trash size={16} />
              Delete scene
            </button>
          </div>
        </section>

        <aside className="inspector">
          <div className="inspector-tabs">
            <button
              className={ui.inspectorTab === "script" ? "active" : ""}
              onClick={() => ui.setInspectorTab("script")}
            >
              SCRIPT
            </button>
            <button
              className={ui.inspectorTab === "scene" ? "active" : ""}
              onClick={() => ui.setInspectorTab("scene")}
            >
              SCENE
            </button>
          </div>
          {providers ? (
            <>
              <label>
                OUTPUT LANGUAGE
                <select
                  value={project.language}
                  onChange={(event) =>
                    setProject((current) => ({ ...current, language: event.target.value }))
                  }
                >
                  {providers.languages.map((language) => (
                    <option key={language}>{language}</option>
                  ))}
                </select>
              </label>
              {providers.translation.length > 0 && (
                <ProviderField
                  title="TRANSLATION PROVIDER"
                  value={project.translationProvider}
                  options={providers.translation}
                  onChange={(value) =>
                    setProject((current) => ({ ...current, translationProvider: value }))
                  }
                />
              )}
              {translationProvider && (
                <div className="provider-status">
                  <Check size={14} weight="bold" />
                  <span>
                    {translationProvider.kind === "local"
                      ? translationProvider.online
                        ? "Local · uses online API"
                        : "Runs on this device"
                      : "Uses your account"}
                  </span>
                  <button onClick={() => ui.setSettingsOpen(true)}>Configure</button>
                </div>
              )}
              {providers.voice.length > 0 && (
                <ProviderField
                  title="VOICE PROVIDER"
                  value={project.voiceProvider}
                  options={providers.voice}
                  onChange={(value) =>
                    setProject((current) => ({ ...current, voiceProvider: value }))
                  }
                />
              )}
              {voiceProvider && (
                <div className="provider-status">
                  <Check size={14} weight="bold" />
                  <span>
                    {voiceProvider.kind === "local"
                      ? voiceProvider.online
                        ? "Free · Microsoft Neural via Python"
                        : "Runs on this device"
                      : "Uses your account"}
                  </span>
                  <button onClick={() => ui.setSettingsOpen(true)}>Configure</button>
                </div>
              )}
              <label>
                VOICE
                <select
                  value={project.voice}
                  onChange={(event) =>
                    setProject((current) => ({ ...current, voice: event.target.value }))
                  }
                >
                  {voiceOptionsFor(project)}
                </select>
              </label>
              <button
                className="preview-voice"
                onClick={handlePreviewVoice}
                disabled={preview.loading}
              >
                <Play size={15} weight="fill" />
                {preview.loading ? "Generating…" : "Preview voice"}
              </button>
              {preview.error && <p className="preview-error">{preview.error}</p>}
              <div className="slider-row">
                <span>Speed</span>
                <input
                  type="range"
                  min="75"
                  max="125"
                  value={project.voiceSpeed}
                  onChange={(e) =>
                    setProject((p) => ({ ...p, voiceSpeed: Number(e.target.value) }))
                  }
                />
                <output>{(project.voiceSpeed / 100).toFixed(2)}×</output>
              </div>
              <div className="local-note">
                <Check size={18} weight="fill" />
                <div>
                  <strong>{ttsReady === false ? "edge-tts not detected" : "edge-tts ready"}</strong>
                  <span>
                    {ttsReady === false
                      ? "Install Python then: pip install edge-tts"
                      : "Microsoft Neural voices via Python. No key required."}
                  </span>
                </div>
              </div>
              {project.translationProvider === "marian" && project.language !== "English (US)" && (
                <div className="local-note warn">
                  <Warning size={18} weight="fill" />
                  <div>
                    <strong>MarianMT translation not yet implemented</strong>
                    <span>
                      Picking a non-English output with MarianMT will keep the source script and
                      show a warning after export. Use OpenAI or Google Cloud for actual
                      translation.
                    </span>
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="inspector-loading">Loading providers…</div>
          )}
          {ui.inspectorTab === "scene" && (
            <div className="scene-meta-panel">
              <label>
                PAGE TITLE
                <input
                  type="text"
                  value={active.title}
                  onChange={(event) => updateScene(active.id, { title: event.target.value })}
                />
              </label>
              <label>
                DURATION (seconds)
                <input
                  type="number"
                  min="1"
                  value={active.duration}
                  onChange={(event) =>
                    updateScene(active.id, { duration: Math.max(1, Number(event.target.value)) })
                  }
                />
              </label>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={active.selected}
                  onChange={(event) => updateScene(active.id, { selected: event.target.checked })}
                />
                <div>
                  <strong>Include this scene</strong>
                  <span>Selected scenes render in the final video</span>
                </div>
              </label>
            </div>
          )}
          <ProviderHealth onOpenModels={() => ui.openModels()} />
          <div className="export-section">
            <span>EXPORT VIDEO</span>
            <label className="check-row">
              <input
                type="checkbox"
                checked={project.outputYouTube}
                onChange={(event) =>
                  setProject((current) => ({ ...current, outputYouTube: event.target.checked }))
                }
              />
              <div>
                <strong>YouTube</strong>
                <span>1920×1080 · H.264</span>
              </div>
            </label>
            <label className="check-row">
              <input
                type="checkbox"
                checked={project.outputTikTok}
                onChange={(event) =>
                  setProject((current) => ({ ...current, outputTikTok: event.target.checked }))
                }
              />
              <div>
                <strong>TikTok</strong>
                <span>1080×1920 · H.264</span>
              </div>
            </label>
            <button className="export-primary" onClick={ui.openExport}>
              <Export size={18} />
              Export video
            </button>
          </div>
        </aside>
      </section>

      <footer className="statusbar">
        <span
          className={`status-dot ${system.ffmpeg || system.ffmpegSidecarReady ? "ready" : "warn"}`}
        />
        <span>{status}</span>
        {modelPrompt.neededModelId && (
          <button
            className="status-action"
            onClick={() => modelPrompt.triggerDownload(modelPrompt.neededModelId!)}
            disabled={modelPrompt.downloading}
          >
            {modelPrompt.downloading ? "Downloading…" : "Download model"}
          </button>
        )}
        <span className="system-status">
          {system.platform} · FFmpeg{" "}
          {system.ffmpeg || system.ffmpegSidecarReady ? "ready" : "not found"}
        </span>
      </footer>

      {ui.settingsOpen && (
        <SettingsModal
          onClose={() => ui.setSettingsOpen(false)}
          onOpenModels={() => {
            ui.setSettingsOpen(false);
            ui.setModelsOpen(true);
          }}
        />
      )}
      {ui.modelsOpen && <ModelsModal onClose={() => ui.setModelsOpen(false)} />}
      {ui.exportOpen && (
        <ExportModal
          project={project}
          onClose={() => ui.setExportOpen(false)}
          onStart={startExport}
          onOpenSettings={() => {
            ui.setExportOpen(false);
            ui.setSettingsOpen(true);
          }}
        />
      )}
      {progressJobId && (
        <ProgressModal jobId={progressJobId} onClose={() => setProgressJobId(null)} />
      )}
      {ui.previewOpen && (
        <PreviewModal
          onClose={() => ui.setPreviewOpen(false)}
          scene={active}
          voiceProvider={project.voiceProvider}
          voice={project.voice}
          scenes={project.scenes}
          onSceneChange={setActiveId}
        />
      )}
    </main>
  );
}

export default App;
