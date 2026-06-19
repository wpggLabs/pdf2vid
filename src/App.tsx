import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowsOut, Check, Export, FilePdf, Gear, List, Pause,
  Play, Plus, SkipBack, SkipForward, SpeakerHigh, Trash, Waveform, Warning,
} from "@phosphor-icons/react";
import "./App.css";
import { parsePdf } from "./pdf";
import { ExportModal } from "./components/ExportModal";
import { SettingsModal } from "./components/SettingsModal";
import { ModelsModal } from "./components/ModelsModal";
import { ProgressModal } from "./components/ProgressModal";
import { ProviderField } from "./components/ProviderField";
import { PreviewModal } from "./components/PreviewModal";
import {
  getProviderList, getSystemStatus, loadProject, saveProject, startExport as backendExport,
} from "./backend";
import { usePreviewVoice } from "./hooks/usePreviewVoice";
import { useTimelinePlayback } from "./hooks/useTimelinePlayback";
import { useTranslationModelPrompt } from "./hooks/useTranslationModelPrompt";
import { voiceOptionsFor } from "./lib/voiceOptions";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { ProviderOption } from "./types";
import type { Scene, Project, SystemStatus } from "./types";
import type { ProviderList } from "./api";

const demoScenes: Scene[] = [
  {
    id: "welcome", page: 1, title: "Start with a PDF",
    script: "Import a PDF to build your first narrated video.", duration: 7,
    selected: true, thumbnail: "",
  },
];

const defaultProject: Project = {
  name: "Untitled project",
  sourceName: "No PDF imported",
  scenes: demoScenes,
  language: "English (US)",
  translationProvider: "marian",
  voiceProvider: "edge",
  voice: "en-US-AriaNeural",
  outputYouTube: true,
  outputTikTok: true,
};

function seconds(value: number) {
  const minutes = Math.floor(value / 60);
  return `${String(minutes).padStart(2, "0")}:${String(value % 60).padStart(2, "0")}`;
}

function providerById(options: ProviderOption[], id: string) {
  return options.find((option) => option.id === id) ?? options[0];
}

type WorkspaceTab = "scenes" | "preview" | "export";
type TimelineTab = "timeline" | "subtitles";
type InspectorTab = "script" | "scene";

function App() {
  const [project, setProject] = useState<Project>(defaultProject);
  const [providers, setProviders] = useState<ProviderList | null>(null);
  const [activeId, setActiveId] = useState(project.scenes[0].id);
  const [aspect, setAspect] = useState<"youtube" | "tiktok">("youtube");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [modelsOpen, setModelsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [progressJobId, setProgressJobId] = useState<string | null>(null);
  const [status, setStatus] = useState("Loading project…");
  const [system, setSystem] = useState<SystemStatus>({
    ffmpeg: false, ffprobe: false, platform: "Browser preview", ffmpegSidecarReady: false,
  });
  const [workspaceTab, setWorkspaceTab] = useState<WorkspaceTab>("scenes");
  const [timelineTab, setTimelineTab] = useState<TimelineTab>("timeline");
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("script");
  const [importProgress, setImportProgress] = useState<{ page: number; total: number } | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const saveTimer = useRef<number | null>(null);
  const previewRef = useRef<HTMLDivElement>(null);
  const importAbort = useRef<AbortController | null>(null);

  const active = project.scenes.find((scene) => scene.id === activeId) ?? project.scenes[0];
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

  // Pass-through setter for status messages from the translation model prompt.
  const setStatusFromPrompt = useCallback(
    (message: string) => setStatus(message),
    [],
  );
  const modelPrompt = useTranslationModelPrompt(
    project.translationProvider,
    project.language,
    setStatusFromPrompt,
  );

  // Initial load + system status
  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const [saved, list, status] = await Promise.all([
          loadProject(),
          getProviderList(),
          getSystemStatus(),
        ]);
        if (!mounted) return;
        if (saved) {
          setProject(saved);
          setActiveId(saved.scenes[0]?.id ?? demoScenes[0].id);
          setStatus("Project loaded");
        } else {
          setStatus("Ready");
        }
        setProviders(list);
        setSystem(status);
        if (!status.ffmpeg && !status.ffmpegSidecarReady) {
          setStatus("Ready · Install FFmpeg or bundle the sidecar to render videos");
        }
      } catch (error) {
        if (mounted) setStatus(`Could not initialize: ${error}`);
      }
    })();
    return () => {
      mounted = false;
    };
  }, []);

  // Refresh system status on window focus so installing FFmpeg while the app is open gets picked up
  useEffect(() => {
    const refresh = () => {
      getSystemStatus()
        .then((s) => {
          setSystem(s);
          if (s.ffmpeg || s.ffmpegSidecarReady) {
            setStatus((prev) =>
              prev.includes("FFmpeg") ? "Ready" : prev,
            );
          }
        })
        .catch(() => undefined);
    };
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, []);

  // Debounced auto-save; flush on unmount
  useEffect(() => {
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveProject(project).catch((error) => setStatus(`Save failed: ${error}`));
    }, 600);
    return () => {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
    };
  }, [project]);

  // Flush save on unmount
  useEffect(() => {
    return () => {
      // Best-effort sync save when component unmounts
      const json = JSON.stringify(project);
      try {
        navigator.sendBeacon?.("/save-project", json);
      } catch {
        // ignore
      }
    };
  }, [project]);

  async function importPdf(file?: File) {
    if (!file) return;
    importAbort.current = new AbortController();
    setStatus("Reading PDF…");
    setImportProgress({ page: 0, total: 0 });
    try {
      const result = await parsePdf(
        { kind: "file", file },
        (page, total) => {
          setStatus(`Reading page ${page} of ${total}`);
          setImportProgress({ page, total });
        },
        importAbort.current.signal,
      );
      const name = file.name.replace(/\.pdf$/i, "");
      setProject((current) => ({ ...current, name, sourceName: file.name, scenes: result.scenes }));
      setActiveId(result.scenes[0].id);
      setStatus(formatImportStatus(result.scenes.length, result.skippedPages));
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Could not read this PDF");
    } finally {
      setImportProgress(null);
      importAbort.current = null;
    }
  }

  async function importPdfFromPath(path: string) {
    importAbort.current = new AbortController();
    setStatus("Reading PDF…");
    setImportProgress({ page: 0, total: 0 });
    try {
      const result = await parsePdf(
        { kind: "path", path },
        (page, total) => {
          setStatus(`Reading page ${page} of ${total}`);
          setImportProgress({ page, total });
        },
        importAbort.current.signal,
      );
      const name = path.split(/[\\/]/).pop()?.replace(/\.pdf$/i, "") ?? "Untitled";
      setProject((current) => ({ ...current, name, sourceName: path, scenes: result.scenes }));
      setActiveId(result.scenes[0].id);
      setStatus(formatImportStatus(result.scenes.length, result.skippedPages));
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Could not read this PDF");
    } finally {
      setImportProgress(null);
      importAbort.current = null;
    }
  }

  function formatImportStatus(imported: number, skipped: number[]): string {
    if (skipped.length === 0) {
      return `${imported} pages imported`;
    }
    const sample = skipped.slice(0, 3).join(", ");
    const more = skipped.length > 3 ? `, +${skipped.length - 3} more` : "";
    return `${imported} pages imported · ${skipped.length} skipped (no text): ${sample}${more}`;
  }

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

  function updateScene(id: string, changes: Partial<Scene>) {
    setProject((current) => ({
      ...current,
      scenes: current.scenes.map((scene) =>
        scene.id === id ? { ...scene, ...changes } : scene,
      ),
    }));
  }

  function removeScene(id: string) {
    setProject((current) => {
      if (current.scenes.length === 1) return current;
      const scenes = current.scenes.filter((scene) => scene.id !== id);
      setActiveId(scenes[0].id);
      return { ...current, scenes };
    });
  }

  async function startExport(jobId: string, outputDir: string) {
    setProgressJobId(jobId);
    setStatus("Export queued");
    try {
      const complete = await backendExport(jobId, project, outputDir);
      const paths = [complete.youtubePath, complete.tiktokPath]
        .filter((p): p is string => Boolean(p))
        .join(", ");
      setStatus(`Export complete · ${paths || "saved"}`);
    } catch (error) {
      setStatus(`Export failed: ${error}`);
    }
  }

  // Skip back / forward = previous / next selected scene
  const selectedScenes = useMemo(
    () => project.scenes.filter((s) => s.selected),
    [project.scenes],
  );
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
      preview.preview(project.voiceProvider, project.voice, active.script).catch(() => undefined);
    }
  }, [playback, preview, project.voiceProvider, project.voice, active.script]);

  // Preview voice (active scene's script)
  const handlePreviewVoice = useCallback(() => {
    preview.preview(project.voiceProvider, project.voice, active.script);
  }, [preview, project.voiceProvider, project.voice, active.script]);

  // Fullscreen toggle
  const toggleFullscreen = useCallback(() => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen?.().catch(() => undefined);
    } else {
      document.exitFullscreen?.().catch(() => undefined);
    }
  }, []);

  // Switch to Preview workspace tab and open the preview modal
  const openPreview = useCallback(() => {
    setPreviewOpen(true);
  }, []);

  const translationProvider = providers
    ? providerById(providers.translation, project.translationProvider)
    : null;
  const voiceProvider = providers
    ? providerById(providers.voice, project.voiceProvider)
    : null;

  const totalDuration = duration;

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand"><span>pdf2</span><strong>vid</strong></div>
        <div className="project-title">
          <span>Projects</span><b>/</b><strong>{project.name}</strong>
        </div>
        <nav aria-label="Workspace">
          <button
            className={workspaceTab === "scenes" ? "nav-active" : ""}
            onClick={() => setWorkspaceTab("scenes")}
          >
            <List size={18} />Scenes
          </button>
          <button
            className={workspaceTab === "preview" ? "nav-active" : ""}
            onClick={openPreview}
          >
            <Play size={18} />Preview
          </button>
          <button onClick={() => setExportOpen(true)}>
            <Export size={18} />Export
          </button>
        </nav>
        <button
          className="icon-button"
          aria-label="Settings"
          onClick={() => setSettingsOpen(true)}
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
            onChange={(event) => importPdf(event.target.files?.[0])}
          />
          <button className="import-button" onClick={pickAndImportPdf}>
            <FilePdf size={20} />Import PDF
          </button>
          {importProgress && (
            <div className="import-progress">
              <span>Reading page {importProgress.page} of {importProgress.total}</span>
              <div className="import-progress-bar">
                <div
                  className="import-progress-bar-fill"
                  style={{ width: `${(importProgress.page / importProgress.total) * 100}%` }}
                />
              </div>
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
              value={aspect}
              onChange={(event) => setAspect(event.target.value as "youtube" | "tiktok")}
            >
              <option value="youtube">YouTube · 1920×1080</option>
              <option value="tiktok">TikTok · 1080×1920</option>
            </select>
            <button className="icon-button" onClick={toggleFullscreen} aria-label="Fullscreen">
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
              <p>{active.script}</p>
            </div>
          </div>
          <div className="transport">
            <span>{seconds(playback.totalElapsed)}</span>
            <div className="transport-actions">
              <button onClick={skipBack} aria-label="Previous scene">
                <SkipBack weight="fill" />
              </button>
              <button className="play" onClick={togglePlay} aria-label={playback.playing ? "Pause" : "Play"}>
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
              className={timelineTab === "timeline" ? "active" : ""}
              onClick={() => setTimelineTab("timeline")}
            >
              TIMELINE
            </button>
            <button
              className={timelineTab === "subtitles" ? "active" : ""}
              onClick={() => setTimelineTab("subtitles")}
            >
              SUBTITLES
            </button>
          </div>
          {timelineTab === "timeline" ? (
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
                {project.scenes.filter((s) => s.selected).map((scene, i) => (
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
              <Trash size={16} />Delete scene
            </button>
          </div>
        </section>

        <aside className="inspector">
          <div className="inspector-tabs">
            <button
              className={inspectorTab === "script" ? "active" : ""}
              onClick={() => setInspectorTab("script")}
            >
              SCRIPT
            </button>
            <button
              className={inspectorTab === "scene" ? "active" : ""}
              onClick={() => setInspectorTab("scene")}
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
                  <button onClick={() => setSettingsOpen(true)}>Configure</button>
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
                  <button onClick={() => setSettingsOpen(true)}>Configure</button>
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
                <input type="range" min="75" max="125" defaultValue="100" />
                <output>1.00×</output>
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
                      Picking a non-English output with MarianMT will keep the source
                      script and show a warning after export. Use OpenAI or Google
                      Cloud for actual translation.
                    </span>
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="inspector-loading">Loading providers…</div>
          )}
          {inspectorTab === "scene" && (
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
                  onChange={(event) =>
                    updateScene(active.id, { selected: event.target.checked })
                  }
                />
                <div>
                  <strong>Include this scene</strong>
                  <span>Selected scenes render in the final video</span>
                </div>
              </label>
            </div>
          )}
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
            <button className="export-primary" onClick={() => setExportOpen(true)}>
              <Export size={18} />Export video
            </button>
          </div>
        </aside>
      </section>

      <footer className="statusbar">
        <span className={`status-dot ${system.ffmpeg || system.ffmpegSidecarReady ? "ready" : "warn"}`} />
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
          {system.platform} · FFmpeg {system.ffmpeg || system.ffmpegSidecarReady ? "ready" : "not found"}
        </span>
      </footer>

      {settingsOpen && (
        <SettingsModal
          onClose={() => setSettingsOpen(false)}
          onOpenModels={() => {
            setSettingsOpen(false);
            setModelsOpen(true);
          }}
        />
      )}
      {modelsOpen && <ModelsModal onClose={() => setModelsOpen(false)} />}
      {exportOpen && (
        <ExportModal
          project={project}
          onClose={() => setExportOpen(false)}
          onStart={startExport}
          onOpenSettings={() => {
            setExportOpen(false);
            setSettingsOpen(true);
          }}
        />
      )}
      {progressJobId && (
        <ProgressModal jobId={progressJobId} onClose={() => setProgressJobId(null)} />
      )}
      {previewOpen && (
        <PreviewModal
          onClose={() => setPreviewOpen(false)}
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