import type { ProviderCategory, ProviderKind, ProviderOption } from "./types";

export interface ProviderList {
  translation: ProviderOption[];
  voice: ProviderOption[];
  visual: ProviderOption[];
  languages: string[];
}

export interface SystemStatus {
  ffmpeg: boolean;
  ffprobe: boolean;
  platform: string;
  ffmpegSidecarReady: boolean;
}

export interface ModelInfo {
  id: string;
  family: string;
  label: string;
  url: string;
  sizeBytes: number;
  sha256: string;
  license: string;
  requiresAccept: boolean;
  installed: boolean;
  path: string | null;
}

export interface ModelDownloadProgress {
  modelId: string;
  downloaded: number;
  total: number;
  percent: number;
}

export interface ExportProgress {
  jobId: string;
  stage: string;
  message: string;
  percent: number;
  current: number | null;
  total: number | null;
}

export interface TranslationWarning {
  sceneId: string;
  page: number;
  provider: string;
  message: string;
}

export interface ExportComplete {
  jobId: string;
  youtubePath: string | null;
  tiktokPath: string | null;
  translationWarnings: TranslationWarning[];
}

export interface ExportError {
  jobId: string;
  stage: string;
  message: string;
}

export type { ProviderOption, ProviderKind, ProviderCategory };