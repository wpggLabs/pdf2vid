export type ProviderKind = "local" | "api";
export type ProviderCategory = "translation" | "voice" | "visual";

export interface ProviderOption {
  id: string;
  label: string;
  kind: ProviderKind;
  detail: string;
  implemented: boolean;
  online: boolean;
  keyLabel: string | null;
  category: ProviderCategory;
}

export interface Scene {
  id: string;
  page: number;
  title: string;
  script: string;
  translatedScript?: string | null;
  duration: number;
  selected: boolean;
  thumbnail: string;
}

export interface Project {
  name: string;
  sourceName: string;
  scenes: Scene[];
  language: string;
  translationProvider: string;
  voiceProvider: string;
  voice: string;
  outputYouTube: boolean;
  outputTikTok: boolean;
  /** Pages skipped during import because they had no selectable text. */
  skippedPages?: number[];
}

export interface SystemStatus {
  ffmpeg: boolean;
  ffprobe: boolean;
  platform: string;
  ffmpegSidecarReady: boolean;
}
