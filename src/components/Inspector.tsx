import { Check, Export, Play } from "@phosphor-icons/react";
import type { ProviderList } from "../api";
import { voiceOptionsFor } from "../lib/voiceOptions";
import type { Project, ProviderOption, Scene } from "../types";
import { ProviderField } from "./ProviderField";
import { ProviderHealth } from "./ProviderHealth";

function providerById(options: ProviderOption[], id: string) {
  return options.find((option) => option.id === id) ?? options[0];
}

interface InspectorProps {
  project: Project;
  setProject: React.Dispatch<React.SetStateAction<Project>>;
  active: Scene;
  providers: ProviderList | null;
  inspectorTab: "script" | "scene";
  ttsReady: boolean | null;
  previewLoading: boolean;
  previewError: string | null;
  onInspectorTab: (tab: "script" | "scene") => void;
  onUpdateScene: (id: string, patch: Partial<Scene>) => void;
  onPreviewVoice: () => void;
  onOpenSettings: () => void;
  onOpenModels: () => void;
  onOpenExport: () => void;
}

export function Inspector({
  project,
  setProject,
  active,
  providers,
  inspectorTab,
  ttsReady,
  previewLoading,
  previewError,
  onInspectorTab,
  onUpdateScene,
  onPreviewVoice,
  onOpenSettings,
  onOpenModels,
  onOpenExport,
}: InspectorProps) {
  const translationProvider = providers
    ? providerById(providers.translation, project.translationProvider)
    : null;
  const voiceProvider = providers ? providerById(providers.voice, project.voiceProvider) : null;

  return (
    <aside className="inspector">
      <div className="inspector-tabs">
        <button
          type="button"
          className={inspectorTab === "script" ? "active" : ""}
          onClick={() => onInspectorTab("script")}
        >
          SCRIPT
        </button>
        <button
          type="button"
          className={inspectorTab === "scene" ? "active" : ""}
          onClick={() => onInspectorTab("scene")}
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
              <button type="button" onClick={onOpenSettings}>
                Configure
              </button>
            </div>
          )}
          {providers.voice.length > 0 && (
            <ProviderField
              title="VOICE PROVIDER"
              value={project.voiceProvider}
              options={providers.voice}
              onChange={(value) => setProject((current) => ({ ...current, voiceProvider: value }))}
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
              <button type="button" onClick={onOpenSettings}>
                Configure
              </button>
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
            type="button"
            className="preview-voice"
            onClick={onPreviewVoice}
            disabled={previewLoading}
          >
            <Play size={15} weight="fill" />
            {previewLoading ? "Generating…" : "Preview voice"}
          </button>
          {previewError && <p className="preview-error">{previewError}</p>}
          <div className="slider-row">
            <span>Speed</span>
            <input
              type="range"
              min="75"
              max="125"
              value={project.voiceSpeed}
              onChange={(e) => setProject((p) => ({ ...p, voiceSpeed: Number(e.target.value) }))}
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
          {project.translationProvider === "argos" && project.language !== "English (US)" && (
            <div className="local-note">
              <Check size={18} weight="fill" />
              <div>
                <strong>Offline translation via Argos</strong>
                <span>
                  Requires <code>pip install argostranslate</code>. The first render downloads a
                  small language pack. If it isn't available, the source text is kept and a warning
                  is shown — switch to OpenAI or Google Cloud for cloud translation.
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
              onChange={(event) => onUpdateScene(active.id, { title: event.target.value })}
            />
          </label>
          <label>
            DURATION (seconds)
            <input
              type="number"
              min="1"
              value={active.duration}
              onChange={(event) =>
                onUpdateScene(active.id, { duration: Math.max(1, Number(event.target.value)) })
              }
            />
          </label>
          <label className="check-row">
            <input
              type="checkbox"
              checked={active.selected}
              onChange={(event) => onUpdateScene(active.id, { selected: event.target.checked })}
            />
            <div>
              <strong>Include this scene</strong>
              <span>Selected scenes render in the final video</span>
            </div>
          </label>
        </div>
      )}
      <ProviderHealth onOpenModels={onOpenModels} />
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
        <button type="button" className="export-primary" onClick={onOpenExport}>
          <Export size={18} />
          Export video
        </button>
      </div>
    </aside>
  );
}
