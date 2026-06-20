import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ExportComplete,
  ExportError,
  ExportProgress,
  ModelDownloadProgress,
  ModelInfo,
  ProviderList,
  SystemStatus,
} from "./api";
import type { Project } from "./types";

export function getSystemStatus(): Promise<SystemStatus> {
  return invoke("system_status");
}

export function getProviderList(): Promise<ProviderList> {
  return invoke("list_providers");
}

export function getModels(): Promise<ModelInfo[]> {
  return invoke("list_models");
}

export async function downloadModel(modelId: string): Promise<string> {
  return invoke("download_model", { modelId });
}

export function deleteModel(modelId: string): Promise<void> {
  return invoke("delete_model", { modelId });
}

export function loadProject(): Promise<Project | null> {
  return invoke("load_project");
}

export function saveProject(project: Project): Promise<void> {
  return invoke("save_project", { project });
}

export function storeApiKey(provider: string, secret: string): Promise<void> {
  return invoke("store_api_key", { provider, secret });
}

export function validateExport(project: Project): Promise<string> {
  return invoke("validate_export", { project });
}

export function startExport(
  jobId: string,
  project: Project,
  outputDir: string,
): Promise<ExportComplete> {
  return invoke("start_export", {
    request: { jobId, project, outputDir },
  });
}

export function cancelExport(): Promise<string | null> {
  return invoke("cancel_export");
}

export function translateText(
  provider: string,
  targetLanguage: string,
  text: string,
): Promise<string> {
  return invoke("translate_text", { provider, targetLanguage, text });
}

export function previewVoice(
  provider: string,
  voice: string,
  text: string,
): Promise<string> {
  return invoke("preview_voice", { provider, voice, text });
}

export async function onExportProgress(
  handler: (payload: ExportProgress) => void,
): Promise<UnlistenFn> {
  return listen<ExportProgress>("export:progress", (event) => handler(event.payload));
}

export async function onExportComplete(
  handler: (payload: ExportComplete) => void,
): Promise<UnlistenFn> {
  return listen<ExportComplete>("export:done", (event) => handler(event.payload));
}

export async function onExportError(
  handler: (payload: ExportError) => void,
): Promise<UnlistenFn> {
  return listen<ExportError>("export:error", (event) => handler(event.payload));
}

export async function onModelProgress(
  handler: (payload: ModelDownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<ModelDownloadProgress>("model:progress", (event) => handler(event.payload));
}

export async function onModelComplete(
  handler: (payload: { modelId: string; success: boolean }) => void,
): Promise<UnlistenFn> {
  return listen<{ modelId: string; success: boolean }>("model:complete", (event) =>
    handler(event.payload),
  );
}

export function cancelModelDownload(): Promise<string | null> {
  return invoke<string | null>("cancel_model_download");
}

export function isModelInstalled(modelId: string): Promise<boolean> {
  return invoke("is_model_installed", { modelId });
}

export function readPdfFile(path: string): Promise<number[]> {
  return invoke<number[]>("read_pdf_file", { path });
}

export interface TtsEngineStatus {
  pythonAvailable: boolean;
  pythonPath: string | null;
  edgeTtsVersion: string | null;
}

export function checkTtsEngine(): Promise<TtsEngineStatus> {
  return invoke("check_tts_engine");
}

export interface InstallHint {
  tool: string;
  message: string;
  command: string;
}

export interface DependencyStatus {
  ffmpeg: boolean;
  ffprobe: boolean;
  ffmpegPath: string | null;
  python: boolean;
  pythonPath: string | null;
  edgeTts: boolean;
  edgeTtsVersion: string | null;
  platform: string;
  installHints: InstallHint[];
}

export function dependencyStatus(): Promise<DependencyStatus> {
  return invoke("dependency_status");
}