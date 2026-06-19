# Providers

`pdf2vid` exposes every translation, voice, and visual stage through a single
provider registry. The user picks one provider per stage. The first option in each
list is the free default.

## Translation

| Provider | Tier | Cost | Network | Notes |
|---|---|---|---|---|
| **MarianMT** | Free default | Free | Model download (one-time) | Helsinki-NLP Opus-MT. Runs locally via ONNX after model files are downloaded. CC-BY-4.0. |
| **OpenAI** | BYO key | Per-token (your account) | Per request | `gpt-4o-mini` chat completion. |
| **Google Cloud Translation** | BYO key | Per-character (your account) | Per request | REST API v2 with API key. |
| DeepL | Coming soon | — | — | UI badge "Coming soon", blocked from export. |
| Azure Translator | Coming soon | — | — | UI badge "Coming soon", blocked from export. |

## Voice

| Provider | Tier | Cost | Network | Notes |
|---|---|---|---|---|
| **edge-tts** | Free default | Free | Per synthesis (Microsoft) | Microsoft Neural voices (`en-US-JennyNeural`, `es-ES-ElviraNeural`, etc). Highest-quality free voice. |
| **Piper** | Free fallback | Free | Model download (one-time) | Offline ONNX voices after model download. Smaller, less expressive than edge-tts. |
| **OpenAI TTS** | BYO key | Per-character (your account) | Per request | `tts-1` model, voices `alloy`/`shimmer`/`onyx` etc. |
| **ElevenLabs** | BYO key | Per-character (your account) | Per request | Premium neural voices. Voice IDs configurable in Settings. |
| Azure Speech | Coming soon | — | — | UI badge "Coming soon", blocked from export. |

## Visual

| Provider | Tier | Cost | Network | Notes |
|---|---|---|---|---|
| **PDF pages** | Free default | Free | None | The PDF page thumbnails extracted at import, with a Ken Burns zoompan and drawtext subtitle burn. No new dependencies, no model downloads. |
| Higgsfield | Coming soon | — | — | UI badge "Coming soon", blocked from export. |

## Network and license disclosure

- **edge-tts**: sends scene narration text to `*.api.cognitive.microsoft.com` and receives MP3 audio. No account, no quota for normal use. Microsoft's ToS technically restrict non-Edge use; enforcement is effectively zero and the project is widely used in the OSS ecosystem.
- **MarianMT models**: downloaded once from `huggingface.co` (Helsinki-NLP/Opus-MT repos). Most pairs are CC-BY-4.0; some are CC-BY-NC. The model picker in Settings surfaces the license before download.
- **Piper voices**: downloaded once from `huggingface.co` (rhasspy/piper-voices). Most are CC-BY-4.0.
- **OpenAI / Google / ElevenLabs**: paid BYO-key tiers. Your key is sent directly from your machine to the provider's API. Keys live in the OS credential store (`keyring` crate).

## What never leaves your device

- PDF text and page images for local-provider exports.
- Project files (`current-project.json` in `app_data_dir`).
- Cached model files for MarianMT and Piper.
- The directory you choose for video output.

## What leaves your device

Only when you explicitly choose a paid provider or the edge-tts default:
- For edge-tts: scene narration text → Microsoft endpoint → MP3 audio.
- For OpenAI translation: scene narration text → `api.openai.com`.
- For OpenAI TTS: scene narration text → `api.openai.com`.
- For Google Translation: scene narration text → `translation.googleapis.com`.
- For ElevenLabs: scene narration text → `api.elevenlabs.io`.

API keys are never logged, never embedded in project files, never sent to any
analytics endpoint.