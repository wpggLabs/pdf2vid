import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowsOut, Check, Export, FilePdf, Gear, List, Pause,
  Play, Plus, SkipBack, SkipForward, SpeakerHigh, Trash, Waveform,
} from "@phosphor-icons/react";
import "./App.css";
import { parsePdf } from "./pdf";
import { ExportModal } from "./components/ExportModal";
import { SettingsModal } from "./components/SettingsModal";
import { ModelsModal } from "./components/ModelsModal";
import { ProgressModal } from "./components/ProgressModal";
import {
  getProviderList, getSystemStatus, loadProject, saveProject, startExport as backendExport,
} from "./backend";
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
  voice: "en-US-JennyNeural",
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

function App() {
  const [project, setProject] = useState<Project>(defaultProject);
  const [providers, setProviders] = useState<ProviderList | null>(null);
  const [activeId, setActiveId] = useState(project.scenes[0].id);
  const [aspect, setAspect] = useState<"youtube" | "tiktok">("youtube");
  const [playing, setPlaying] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [modelsOpen, setModelsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [progressJobId, setProgressJobId] = useState<string | null>(null);
  const [status, setStatus] = useState("Loading project…");
  const [system, setSystem] = useState<SystemStatus>({
    ffmpeg: false, ffprobe: false, platform: "Browser preview", ffmpegSidecarReady: false,
  });
  const inputRef = useRef<HTMLInputElement>(null);
  const saveTimer = useRef<number | null>(null);

  const active = project.scenes.find((scene) => scene.id === activeId) ?? project.scenes[0];
  const duration = useMemo(
    () =>
      project.scenes
        .filter((scene) => scene.selected)
        .reduce((sum, scene) => sum + scene.duration, 0),
    [project.scenes],
  );

  // Initial load
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

  // Debounced auto-save
  useEffect(() => {
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveProject(project).catch((error) => setStatus(`Save failed: ${error}`));
    }, 600);
    return () => {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
    };
  }, [project]);

  async function importPdf(file?: File) {
    if (!file) return;
    setStatus("Reading PDF…");
    try {
      const scenes = await parsePdf(file, (page, total) =>
        setStatus(`Reading page ${page} of ${total}`),
      );
      const name = file.name.replace(/\.pdf$/i, "");
      setProject((current) => ({ ...current, name, sourceName: file.name, scenes }));
      setActiveId(scenes[0].id);
      setStatus(`${scenes.length} pages imported`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Could not read this PDF");
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

  const translationProvider = providers
    ? providerById(providers.translation, project.translationProvider)
    : null;
  const voiceProvider = providers
    ? providerById(providers.voice, project.voiceProvider)
    : null;

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand"><span>pdf2</span><strong>vid</strong></div>
        <div className="project-title">
          <span>Projects</span><b>/</b><strong>{project.name}</strong>
        </div>
        <nav aria-label="Workspace">
          <button className="nav-active"><List size={18} />Scenes</button>
          <button><Play size={18} />Preview</button>
          <button onClick={() => setExportOpen(true)}><Export size={18} />Export</button>
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
          <button className="import-button" onClick={() => inputRef.current?.click()}>
            <FilePdf size={20} />Import PDF
          </button>
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

        <section className="editor">
          <div className="preview-toolbar">
            <select
              value={aspect}
              onChange={(event) => setAspect(event.target.value as "youtube" | "tiktok")}
            >
              <option value="youtube">YouTube · 1920×1080</option>
              <option value="tiktok">TikTok · 1080×1920</option>
            </select>
            <button className="icon-button"><ArrowsOut size={18} /></button>
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
            <span>00:00</span>
            <div className="transport-actions">
              <button><SkipBack weight="fill" /></button>
              <button className="play" onClick={() => setPlaying((value) => !value)}>
                {playing ? <Pause weight="fill" /> : <Play weight="fill" />}
              </button>
              <button><SkipForward weight="fill" /></button>
            </div>
            <span>{seconds(active.duration)}</span>
            <SpeakerHigh size={18} />
          </div>
          <div className="timeline-tabs">
            <button className="active">TIMELINE</button>
            <button>SUBTITLES</button>
          </div>
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
            <button className="active">SCRIPT</button>
            <button>SCENE</button>
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
                  {providers.languages.map((language: string) => (
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
                        ? "Free · uses Microsoft online synthesis"
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
              <button className="preview-voice"><Play size={15} weight="fill" />Preview voice</button>
              <div className="slider-row">
                <span>Speed</span>
                <input type="range" min="75" max="125" defaultValue="100" />
                <output>1.00×</output>
              </div>
              <div className="local-note">
                <Check size={18} weight="fill" />
                <div>
                  <strong>Local defaults available</strong>
                  <span>Free providers keep your document on this device.</span>
                </div>
              </div>
            </>
          ) : (
            <div className="inspector-loading">Loading providers…</div>
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
        <span className="status-dot" />
        <span>{status}</span>
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
    </main>
  );
}

function voiceOptionsFor(project: Project): React.ReactElement[] {
  if (project.voiceProvider === "edge") {
    return [
      <option key="en-US-JennyNeural">Jenny · English (US)</option>,
      <option key="en-US-GuyNeural">Guy · English (US)</option>,
      <option key="es-ES-ElviraNeural">Elvira · Spanish</option>,
      <option key="fr-FR-DeniseNeural">Denise · French</option>,
      <option key="de-DE-KatjaNeural">Katja · German</option>,
      <option key="hi-IN-SwaraNeural">Swara · Hindi</option>,
      <option key="ja-JP-NanamiNeural">Nanami · Japanese</option>,
      <option key="ko-KR-SunHiNeural">SunHi · Korean</option>,
      <option key="zh-CN-XiaoxiaoNeural">Xiaoxiao · Chinese</option>,
      <option key="ar-EG-SalmaNeural">Salma · Arabic</option>,
    ];
  }
  if (project.voiceProvider === "piper") {
    return [
      <option key="piper-amy">Amy · English (US)</option>,
      <option key="piper-ryan">Ryan · English (US)</option>,
    ];
  }
  if (project.voiceProvider === "elevenlabs") {
    return [
      <option key="eleven-rachel">Rachel · ElevenLabs</option>,
      <option key="eleven-domi">Domi · ElevenLabs</option>,
    ];
  }
  if (project.voiceProvider === "openai") {
    return [
      <option key="openai-alloy">Alloy · OpenAI</option>,
      <option key="openai-shimmer">Shimmer · OpenAI</option>,
      <option key="openai-onyx">Onyx · OpenAI</option>,
    ];
  }
  return [<option key="default">Default voice</option>];
}

function ProviderField({
  title, value, options, onChange,
}: {
  title: string;
  value: string;
  options: ProviderOption[];
  onChange: (value: string) => void;
}) {
  return (
    <label>
      {title}
      <select value={value} onChange={(event) => onChange(event.target.value)}>
        {options.map((provider) => (
          <option
            key={`${provider.category}-${provider.id}`}
            value={provider.id}
            disabled={!provider.implemented}
          >
            {provider.label} · {provider.kind === "local" ? "Free" : "BYO key"}
            {!provider.implemented ? " · Coming soon" : ""}
          </option>
        ))}
      </select>
    </label>
  );
}

export default App;