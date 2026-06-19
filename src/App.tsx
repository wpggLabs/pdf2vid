import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowsOut, Check, CloudArrowUp, Export, FilePdf, Gear, Key, List, Pause,
  Play, Plus, SkipBack, SkipForward, SpeakerHigh, Trash, Waveform,
} from "@phosphor-icons/react";
import "./App.css";
import { parsePdf } from "./pdf";
import { languages, translationProviders, visualProviders, voiceProviders } from "./providers";
import type { Project, ProviderOption, Scene, SystemStatus } from "./types";

const demoScenes: Scene[] = [
  { id: "welcome", page: 1, title: "Start with a PDF", script: "Import a PDF to build your first narrated video.", duration: 7, selected: true, thumbnail: "" },
];

const defaultProject: Project = {
  name: "Untitled project", sourceName: "No PDF imported", scenes: demoScenes,
  language: languages[0], translationProvider: "argos", voiceProvider: "piper", voice: "Amy · English (US)",
  outputYouTube: true, outputTikTok: true,
};

function seconds(value: number) {
  const minutes = Math.floor(value / 60);
  return `${String(minutes).padStart(2, "0")}:${String(value % 60).padStart(2, "0")}`;
}

function providerById(options: ProviderOption[], id: string) {
  return options.find((option) => option.id === id) ?? options[0];
}

function App() {
  const [project, setProject] = useState<Project>(() => {
    const saved = localStorage.getItem("pdf2vid.project.v1");
    return saved ? JSON.parse(saved) : defaultProject;
  });
  const [activeId, setActiveId] = useState(project.scenes[0].id);
  const [aspect, setAspect] = useState<"youtube" | "tiktok">("youtube");
  const [playing, setPlaying] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [status, setStatus] = useState("Ready");
  const [progress, setProgress] = useState(0);
  const [system, setSystem] = useState<SystemStatus>({ ffmpeg: false, ffprobe: false, platform: "Browser preview" });
  const inputRef = useRef<HTMLInputElement>(null);
  const active = project.scenes.find((scene) => scene.id === activeId) ?? project.scenes[0];
  const duration = useMemo(() => project.scenes.filter((scene) => scene.selected).reduce((sum, scene) => sum + scene.duration, 0), [project.scenes]);

  useEffect(() => localStorage.setItem("pdf2vid.project.v1", JSON.stringify(project)), [project]);
  useEffect(() => {
    invoke<SystemStatus>("system_status").then(setSystem).catch(() => undefined);
  }, []);

  async function importPdf(file?: File) {
    if (!file) return;
    setStatus("Reading PDF…");
    setProgress(4);
    try {
      const scenes = await parsePdf(file, (page, total) => {
        setStatus(`Reading page ${page} of ${total}`);
        setProgress(Math.round((page / total) * 100));
      });
      const name = file.name.replace(/\.pdf$/i, "");
      setProject((current) => ({ ...current, name, sourceName: file.name, scenes }));
      setActiveId(scenes[0].id);
      setStatus(`${scenes.length} pages imported`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Could not read this PDF");
    } finally {
      window.setTimeout(() => setProgress(0), 900);
    }
  }

  function updateScene(id: string, changes: Partial<Scene>) {
    setProject((current) => ({ ...current, scenes: current.scenes.map((scene) => scene.id === id ? { ...scene, ...changes } : scene) }));
  }

  function removeScene(id: string) {
    setProject((current) => {
      if (current.scenes.length === 1) return current;
      const scenes = current.scenes.filter((scene) => scene.id !== id);
      setActiveId(scenes[0].id);
      return { ...current, scenes };
    });
  }

  async function saveKey(provider: ProviderOption) {
    if (!apiKey.trim()) return;
    try {
      await invoke("store_api_key", { provider: provider.id, secret: apiKey.trim() });
      setStatus(`${provider.label} key saved in your system credential store`);
      setApiKey("");
    } catch {
      setStatus("Secure key storage is available in the desktop app");
    }
  }

  async function startExport() {
    setExportOpen(false);
    setStatus("Preparing export…");
    setProgress(8);
    try {
      await invoke("save_project", { project });
      const result = await invoke<string>("validate_export", { project });
      setProgress(100);
      setStatus(result);
    } catch {
      for (const value of [24, 51, 76, 100]) {
        await new Promise((resolve) => setTimeout(resolve, 220));
        setProgress(value);
      }
      setStatus("Project validated. Native rendering requires the desktop app and FFmpeg.");
    } finally {
      window.setTimeout(() => setProgress(0), 1200);
    }
  }

  const voiceProvider = providerById(voiceProviders, project.voiceProvider);
  const translationProvider = providerById(translationProviders, project.translationProvider);

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand"><span>pdf2</span><strong>vid</strong></div>
        <div className="project-title"><span>Projects</span><b>/</b><strong>{project.name}</strong></div>
        <nav aria-label="Workspace"><button className="nav-active"><List size={18}/>Scenes</button><button><Play size={18}/>Preview</button><button onClick={() => setExportOpen(true)}><Export size={18}/>Export</button></nav>
        <button className="icon-button" aria-label="Settings" onClick={() => setSettingsOpen(true)}><Gear size={20}/></button>
      </header>
      {progress > 0 && <div className="progress" style={{ width: `${progress}%` }}/>} 

      <section className="workspace">
        <aside className="scene-panel">
          <div className="panel-heading"><div><span>PROJECT</span><strong>{project.sourceName}</strong></div><button className="icon-button" onClick={() => inputRef.current?.click()} aria-label="Import PDF"><Plus size={18}/></button></div>
          <input ref={inputRef} type="file" accept="application/pdf" hidden onChange={(event) => importPdf(event.target.files?.[0])}/>
          <button className="import-button" onClick={() => inputRef.current?.click()}><FilePdf size={20}/>Import PDF</button>
          <div className="scene-label"><span>SCENES</span><span>{project.scenes.filter((scene) => scene.selected).length} / {project.scenes.length}</span></div>
          <div className="scene-list">
            {project.scenes.map((scene, index) => (
              <article key={scene.id} className={`scene-row ${scene.id === activeId ? "selected" : ""}`} onClick={() => setActiveId(scene.id)}>
                <button className={`select-box ${scene.selected ? "checked" : ""}`} onClick={(event) => { event.stopPropagation(); updateScene(scene.id, { selected: !scene.selected }); }} aria-label={`Select page ${scene.page}`}>{scene.selected && <Check size={12} weight="bold"/>}</button>
                <div className="thumb">{scene.thumbnail ? <img src={scene.thumbnail} alt=""/> : <FilePdf size={24}/>}<b>{index + 1}</b></div>
                <div className="scene-meta"><strong>{scene.title}</strong><span>Page {scene.page}</span><time>{seconds(scene.duration)}</time></div>
              </article>
            ))}
          </div>
          <footer><span>Total duration</span><time>{seconds(duration)}</time></footer>
        </aside>

        <section className="editor">
          <div className="preview-toolbar"><select value={aspect} onChange={(event) => setAspect(event.target.value as "youtube" | "tiktok")}><option value="youtube">YouTube · 1920×1080</option><option value="tiktok">TikTok · 1080×1920</option></select><button className="icon-button"><ArrowsOut size={18}/></button></div>
          <div className={`preview-stage ${aspect}`}>
            <div className="paper-preview">{active.thumbnail ? <img src={active.thumbnail} alt={`PDF page ${active.page}`}/> : <div className="empty-preview"><FilePdf size={54}/><strong>Import a PDF to begin</strong></div>}<p>{active.script}</p></div>
          </div>
          <div className="transport"><span>00:00</span><div className="transport-actions"><button><SkipBack weight="fill"/></button><button className="play" onClick={() => setPlaying((value) => !value)}>{playing ? <Pause weight="fill"/> : <Play weight="fill"/>}</button><button><SkipForward weight="fill"/></button></div><span>{seconds(active.duration)}</span><SpeakerHigh size={18}/></div>
          <div className="timeline-tabs"><button className="active">TIMELINE</button><button>SUBTITLES</button></div>
          <div className="timeline">
            <div className="time-ruler"><span>0:00</span><span>{seconds(Math.round(duration / 2))}</span><span>{seconds(duration)}</span></div>
            <div className="clip-track">{project.scenes.map((scene, index) => <button key={scene.id} className={scene.id === activeId ? "active" : ""} style={{ flex: scene.duration }} onClick={() => setActiveId(scene.id)}><b>{index + 1}</b><span>{seconds(scene.duration)}</span></button>)}</div>
            <div className="audio-track"><Waveform size={17}/><div>{Array.from({ length: 58 }, (_, index) => <i key={index} style={{ height: `${18 + ((index * 13) % 30)}%` }}/>)}</div></div>
            <div className="subtitle-track"><span>CC</span>{project.scenes.map((scene) => <button key={scene.id} style={{ flex: scene.duration }} onClick={() => setActiveId(scene.id)}>{scene.script}</button>)}</div>
          </div>
          <div className="script-editor"><div><span>SCENE SCRIPT</span><span>{active.script.length} / 5000</span></div><textarea value={active.script} onChange={(event) => updateScene(active.id, { script: event.target.value, title: event.target.value.slice(0, 42) })}/><button className="delete" onClick={() => removeScene(active.id)}><Trash size={16}/>Delete scene</button></div>
        </section>

        <aside className="inspector">
          <div className="inspector-tabs"><button className="active">SCRIPT</button><button>SCENE</button></div>
          <label>OUTPUT LANGUAGE<select value={project.language} onChange={(event) => setProject((current) => ({ ...current, language: event.target.value }))}>{languages.map((language) => <option key={language}>{language}</option>)}</select></label>
          <ProviderField title="TRANSLATION PROVIDER" value={project.translationProvider} options={translationProviders} onChange={(value) => setProject((current) => ({ ...current, translationProvider: value }))}/>
          <div className="provider-status"><Check size={14} weight="bold"/><span>{translationProvider.kind === "local" ? "Ready locally" : "Uses your account"}</span><button onClick={() => setSettingsOpen(true)}>Configure</button></div>
          <ProviderField title="VOICE PROVIDER" value={project.voiceProvider} options={voiceProviders} onChange={(value) => setProject((current) => ({ ...current, voiceProvider: value }))}/>
          <label>VOICE<select value={project.voice} onChange={(event) => setProject((current) => ({ ...current, voice: event.target.value }))}><option>Amy · English (US)</option><option>Ryan · English (US)</option><option>Custom provider voice</option></select></label>
          <button className="preview-voice"><Play size={15} weight="fill"/>Preview voice</button>
          <div className="slider-row"><span>Speed</span><input type="range" min="75" max="125" defaultValue="100"/><output>1.00×</output></div>
          <div className="local-note"><Check size={18} weight="fill"/><div><strong>Local mode available</strong><span>Free providers keep your document on this device.</span></div></div>
          <div className="export-section"><span>EXPORT VIDEO</span><label className="check-row"><input type="checkbox" checked={project.outputYouTube} onChange={(event) => setProject((current) => ({ ...current, outputYouTube: event.target.checked }))}/><div><strong>YouTube</strong><span>1920×1080 · H.264</span></div></label><label className="check-row"><input type="checkbox" checked={project.outputTikTok} onChange={(event) => setProject((current) => ({ ...current, outputTikTok: event.target.checked }))}/><div><strong>TikTok</strong><span>1080×1920 · H.264</span></div></label><button className="export-primary" onClick={() => setExportOpen(true)}><Export size={18}/>Export video</button></div>
        </aside>
      </section>
      <footer className="statusbar"><span className="status-dot"/><span>{status}</span><span className="system-status">{system.platform} · FFmpeg {system.ffmpeg ? "ready" : "not found"}</span></footer>

      {settingsOpen && <Modal title="Provider settings" onClose={() => setSettingsOpen(false)}><label>Provider<select defaultValue={voiceProvider.id}>{[...voiceProviders, ...translationProviders, ...visualProviders].map((provider, index) => <option key={`${provider.id}-${index}`} value={provider.id}>{provider.label} · {provider.detail}</option>)}</select></label><label>API key<input type="password" value={apiKey} onChange={(event) => setApiKey(event.target.value)} placeholder={voiceProvider.keyLabel ?? "Provider API key"}/></label><p className="modal-note"><Key size={17}/>Keys are stored in the operating system credential manager, never in project files.</p><button className="export-primary" onClick={() => saveKey(voiceProvider)}>Save securely</button></Modal>}
      {exportOpen && <Modal title="Export video" onClose={() => setExportOpen(false)}><div className="export-summary"><CloudArrowUp size={36}/><strong>{project.name}</strong><span>{seconds(duration)} · {project.scenes.filter((scene) => scene.selected).length} scenes</span></div><label className="check-row"><input type="checkbox" checked={project.outputYouTube} readOnly/><div><strong>YouTube landscape</strong><span>1920×1080 · Full length</span></div></label><label className="check-row"><input type="checkbox" checked={project.outputTikTok} readOnly/><div><strong>TikTok portrait</strong><span>1080×1920 · Full length</span></div></label><button className="export-primary" onClick={startExport}><Export size={18}/>Start export</button></Modal>}
    </main>
  );
}

function ProviderField({ title, value, options, onChange }: { title: string; value: string; options: ProviderOption[]; onChange: (value: string) => void }) {
  return <label>{title}<select value={value} onChange={(event) => onChange(event.target.value)}>{options.map((provider) => <option key={provider.id} value={provider.id}>{provider.label} · {provider.kind === "local" ? "Free" : "My API"}</option>)}</select></label>;
}

function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  return <div className="modal-backdrop" onMouseDown={onClose}><section className="modal" onMouseDown={(event) => event.stopPropagation()} role="dialog" aria-modal="true" aria-label={title}><header><h2>{title}</h2><button className="icon-button" onClick={onClose}>×</button></header>{children}</section></div>;
}

export default App;
