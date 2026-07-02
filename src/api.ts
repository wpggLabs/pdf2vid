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
  /**
   * True when a usable drawtext font was discovered. When false, exports
   * fall back to text-less drawtext and the `warnings` array on
   * `ExportComplete` carries a `renderFallback` warning.
   */
  fontAvailable?: boolean;
  /** Path to the font that will be passed to FFmpeg, when one exists. */
  fontPath?: string | null;
}

export type WarningCode =
  | "skippedPage"
  | "untranslatedScene"
  | "missingFont"
  | "renderFallback"
  | "missingDependency"
  | "unsupportedProvider"
  | "voiceSynthesisFailed";

export type WarningSeverity = "info" | "warning" | "error";

export interface ProjectWarning {
  code: WarningCode;
  severity: WarningSeverity;
  sceneId?: string | null;
  page?: number | null;
  message: string;
  /** Short technical detail (e.g. an FFmpeg stderr line). */
  detail?: string | null;
  /** Actionable fix suggestion. */
  suggestedFix?: string | null;
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
  skippedPages: number[];
  untranslatedCount: number;
  /** Typed warnings for every category: skipped pages, missing fonts,
   * render fallbacks, dependency issues, etc. */
  warnings?: ProjectWarning[];
  /** True when render fell back to text-less drawtext because no font
   * was found on the host. */
  renderFallbackUsed?: boolean;
}

export interface ExportError {
  jobId: string;
  stage: string;
  message: string;
}

export type { ProviderCategory, ProviderKind, ProviderOption };
