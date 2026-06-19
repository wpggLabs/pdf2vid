export type ProviderKind = "local" | "api";

export interface ProviderOption {
  id: string;
  label: string;
  kind: ProviderKind;
  detail: string;
  keyLabel?: string;
}

export interface Scene {
  id: string;
  page: number;
  title: string;
  script: string;
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
}

export interface SystemStatus {
  ffmpeg: boolean;
  ffprobe: boolean;
  platform: string;
}
