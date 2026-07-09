import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { ProviderList } from "./api";
import { startExport as backendExport, getProviderList, getSystemStatus } from "./backend";
import { EditorSection } from "./components/EditorSection";
import { ExportModal } from "./components/ExportModal";
import { Inspector } from "./components/Inspector";
import { ModelsModal } from "./components/ModelsModal";
import { PreviewModal } from "./components/PreviewModal";
import { ProgressModal } from "./components/ProgressModal";
import { ScenePanel } from "./components/ScenePanel";
import { SettingsModal } from "./components/SettingsModal";
import { TopBar } from "./components/TopBar";
import { usePreviewVoice } from "./hooks/usePreviewVoice";
import { useTimelinePlayback } from "./hooks/useTimelinePlayback";
import { useTranslationModelPrompt } from "./hooks/useTranslationModelPrompt";
import { captionLineAt, wrapCaptionLines } from "./lib/captions";
import { useProjectState } from "./state/useProjectState";
import { useWorkspaceUi } from "./state/useWorkspaceUi";
import type { SystemStatus } from "./types";

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

  const duration = useMemo(
    () =>
      project.scenes
        .filter((scene) => scene.selected)
        .reduce((sum, scene) => sum + scene.duration, 0),
    [project.scenes],
  );

  const playback = useTimelinePlayback(project.scenes, activeId, setActiveId);
  const preview = usePreviewVoice();
  const [ttsReady, setTtsReady] = useState<boolean | null>(null);

  // Check whether Python + edge-tts is available on startup, and kick off
  // a background OCR engine install so scanned-PDF reading works without
  // the user installing anything manually.
  useEffect(() => {
    import("./backend").then(({ checkTtsEngine, ensureOcr }) => {
      checkTtsEngine()
        .then((status) => setTtsReady(status.pythonAvailable))
        .catch(() => setTtsReady(false));
      ensureOcr().catch(() => undefined);
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
  }, [setStatus]);

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
  }, [setStatus]);

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
  }, [selectedIndex, selectedScenes, setActiveId]);

  const skipForward = useCallback(() => {
    const next = selectedScenes[Math.min(selectedScenes.length - 1, selectedIndex + 1)];
    if (next) setActiveId(next.id);
  }, [selectedIndex, selectedScenes, setActiveId]);

  // Play / pause the timeline simulation
  const togglePlay = useCallback(() => {
    if (playback.playing) {
      playback.pause();
      // Stop the narration audio too, otherwise the voice keeps playing
      // after the visual timeline is paused.
      preview.stop();
    } else {
      playback.play();
    }
  }, [playback, preview]);

  // While playing, follow the active scene with its narration audio so the
  // voice advances in step with the page/caption (not just on first play).
  // When paused, halt the audio so the transport fully stops.
  const followedSceneId = useRef<string | null>(null);
  useEffect(() => {
    if (!playback.playing) {
      followedSceneId.current = null;
      preview.stop();
      return;
    }
    if (followedSceneId.current === active.id) return;
    followedSceneId.current = active.id;
    const sceneId = active.id;
    preview
      .preview(project.voiceProvider, project.voice, active.script, project.voiceSpeed)
      .then((audioSeconds) => {
        // Adopt the narration's real length as the scene duration (like
        // the exporter does via ffprobe) so the preview timeline, captions
        // and the final video all agree. Only update on a meaningful
        // difference to avoid churning saves over sub-second jitter.
        if (audioSeconds === null) return;
        const scene = project.scenes.find((s) => s.id === sceneId);
        if (scene && Math.abs(scene.duration - audioSeconds) > 1) {
          updateScene(sceneId, { duration: Math.max(1, Math.ceil(audioSeconds)) });
        }
      })
      .catch(() => undefined);
  }, [
    playback.playing,
    active.id,
    active.script,
    project.voiceProvider,
    project.voice,
    project.voiceSpeed,
    preview,
    project.scenes,
    updateScene,
  ]);

  // Preview voice (active scene's script)
  const handlePreviewVoice = useCallback(() => {
    preview.preview(project.voiceProvider, project.voice, active.script, project.voiceSpeed);
  }, [preview, project.voiceProvider, project.voice, active.script, project.voiceSpeed]);

  // Read-along preview caption: show the single line that matches the
  // current playback position (mirrors the line-by-line export) instead
  // of dumping the whole script over the page.
  const captionLines = useMemo(() => wrapCaptionLines(active.script), [active.script]);
  const previewCaption = useMemo(() => {
    const progress = playback.playing ? playback.elapsedInScene / Math.max(1, active.duration) : 0;
    return captionLineAt(captionLines, progress);
  }, [captionLines, playback.playing, playback.elapsedInScene, active.duration]);

  return (
    <main className="app-shell">
      <TopBar
        projectName={project.name}
        workspaceTab={ui.workspaceTab}
        onScenesTab={() => ui.setWorkspaceTab("scenes")}
        onPreviewTab={ui.handlePreviewTab}
        onExport={ui.openExport}
        onSettings={() => ui.setSettingsOpen(true)}
      />

      <section className="workspace">
        <ScenePanel
          sourceName={project.sourceName}
          scenes={project.scenes}
          activeId={activeId}
          totalDuration={duration}
          importProgress={importProgress}
          importSummary={importSummary}
          onImportFile={importPdf}
          onPickPdf={pickAndImportPdf}
          onSelectScene={setActiveId}
          onToggleScene={(id, selected) => updateScene(id, { selected })}
        />

        <EditorSection
          scenes={project.scenes}
          active={active}
          activeId={activeId}
          aspect={ui.aspect}
          timelineTab={ui.timelineTab}
          playing={playback.playing}
          totalElapsed={playback.totalElapsed}
          totalDuration={duration}
          previewCaption={previewCaption}
          onAspectChange={ui.setAspect}
          onTimelineTab={ui.setTimelineTab}
          onToggleFullscreen={ui.toggleFullscreen}
          onSelectScene={setActiveId}
          onSkipBack={skipBack}
          onSkipForward={skipForward}
          onTogglePlay={togglePlay}
          onScriptChange={(script) =>
            updateScene(active.id, { script, title: script.slice(0, 42) })
          }
          onDeleteScene={() => removeScene(active.id)}
        />

        <Inspector
          project={project}
          setProject={setProject}
          active={active}
          providers={providers}
          inspectorTab={ui.inspectorTab}
          ttsReady={ttsReady}
          previewLoading={preview.loading}
          previewError={preview.error}
          onInspectorTab={ui.setInspectorTab}
          onUpdateScene={updateScene}
          onPreviewVoice={handlePreviewVoice}
          onOpenSettings={() => ui.setSettingsOpen(true)}
          onOpenModels={() => ui.openModels()}
          onOpenExport={ui.openExport}
        />
      </section>

      <footer className="statusbar">
        <span
          className={`status-dot ${system.ffmpeg || system.ffmpegSidecarReady ? "ready" : "warn"}`}
        />
        <span>{status}</span>
        {modelPrompt.neededModelId && (
          <button
            type="button"
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
