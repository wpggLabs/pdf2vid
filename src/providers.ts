import type { ProviderOption } from "./types";

export const translationProviders: ProviderOption[] = [
  { id: "argos", label: "Argos Translate", kind: "local", detail: "Free · Runs on this device" },
  { id: "deepl", label: "DeepL", kind: "api", detail: "Use your API key", keyLabel: "DeepL API key" },
  { id: "google", label: "Google Cloud", kind: "api", detail: "Use your API key", keyLabel: "Google Cloud API key" },
  { id: "openai", label: "OpenAI", kind: "api", detail: "Use your API key", keyLabel: "OpenAI API key" },
];

export const voiceProviders: ProviderOption[] = [
  { id: "piper", label: "Piper", kind: "local", detail: "Free · Runs on this device" },
  { id: "elevenlabs", label: "ElevenLabs", kind: "api", detail: "Use your API key", keyLabel: "ElevenLabs API key" },
  { id: "openai", label: "OpenAI", kind: "api", detail: "Use your API key", keyLabel: "OpenAI API key" },
  { id: "azure", label: "Azure Speech", kind: "api", detail: "Use your API key", keyLabel: "Azure Speech key" },
];

export const visualProviders: ProviderOption[] = [
  { id: "pages", label: "PDF pages", kind: "local", detail: "Free · Original document" },
  { id: "higgsfield", label: "Higgsfield", kind: "api", detail: "Use your API key", keyLabel: "Higgsfield API key" },
];

export const languages = ["English (US)", "Spanish", "French", "German", "Portuguese", "Hindi", "Japanese", "Korean", "Chinese (Simplified)", "Arabic"];
