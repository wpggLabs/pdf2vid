# Providers

`pdf2vid` exposes every translation, voice, and visual stage through a
single provider registry. The user picks one provider per stage. The
first option in each list is the free default.

## Translation

| Provider | Tier | Cost | Network | Notes |
|---|---|---|---|---|
| **Argos Translate** | Free default | Free | Language pack download (one-time) | Offline OpenNMT/CTranslate2 translator (MIT). `pip install argostranslate`; the language pack for a pair auto-downloads on first use. |
| **OpenAI** | BYO key | Per-token (your account) | Per request | `gpt-4o-mini` chat completion. |
| **Google Cloud Translation** | BYO key | Per-character (your account) | Per request | REST API v2 with API key. |
| DeepL | Coming soon | — | — | UI badge "Coming soon", blocked from export. |
| Azure Translator | Coming soon | — | — | UI badge "Coming soon", blocked from export. |

## Voice

| Provider | Tier | Cost | Network | Notes |
|---|---|---|---|---|
| **edge-tts** | Free default | Free | Per synthesis (Microsoft) | Microsoft Neural voices (`en-US-AriaNeural`, `en-US-JennyNeural`, etc). Shells out to the `edge-tts` Python package (`pip install edge-tts`). Also emits word-level subtitles used for read-along captions. |
| **Kokoro** | Free · local | Free | Model download (one-time) | 82M Apache-2.0 model, 8 languages, fast on CPU/GPU. `pip install kokoro soundfile`. |
| **Chatterbox** | Free · local | Free | Model download (one-time) | MIT multilingual model, 23 languages, expressive (GPU recommended). `pip install chatterbox-tts torchaudio`. |
| **Piper** | Free · local | Free | Model download (one-time) | Offline ONNX voices. Registry entry present; ONNX inference is a stub. |
| **OpenAI TTS** | BYO key | Per-character (your account) | Per request | `tts-1` model, voices `alloy`/`shimmer`/`onyx` etc. |
| **ElevenLabs** | BYO key | Per-character (your account) | Per request | Premium neural voices. Voice IDs configurable in Settings. |
| Azure Speech | Coming soon | — | — | UI badge "Coming soon", blocked from export. |

## Visual

| Provider | Tier | Cost | Network | Notes |
|---|---|---|---|---|
| **PDF pages** | Free default | Free | None | The PDF page thumbnails extracted at import, with a Ken Burns zoompan and drawtext subtitle burn. No new dependencies, no model downloads. |
| Higgsfield | Coming soon | — | — | UI badge "Coming soon", blocked from export. |

## Network and license disclosure

- **edge-tts** — sends scene narration text to
  `*.api.cognitive.microsoft.com` and receives MP3 audio. No account, no
  quota for normal use. Microsoft's ToS technically restrict non-Edge use;
  enforcement is effectively zero and the project is widely used in the
  OSS ecosystem. Requires Python 3.8+ with the `edge-tts` package
  (`pip install edge-tts`).

- **Argos language packs** — downloaded once from the Argos package
  index on first translation of a pair (MIT).

- **Kokoro / Chatterbox** — model weights downloaded once from
  `huggingface.co` on first synthesis. Kokoro is Apache-2.0, Chatterbox
  is MIT. Download progress streams into the export progress modal.

- **OpenAI / Google / ElevenLabs** — paid BYO-key tiers. Your key is
  sent directly from your machine to the provider's API. Keys live in
  the OS credential store (`keyring` crate).

## What never leaves your device

- PDF text and page images for local-provider exports.
- Project files (`current-project.json` in `app_data_dir`).
- Cached model files for Argos, Kokoro, and Chatterbox.
- The directory you choose for video output.

## What leaves your device

Only when you explicitly choose a paid provider or the edge-tts default:

- For edge-tts: scene narration text → Microsoft endpoint → MP3 audio
- For OpenAI translation: scene narration text → `api.openai.com`
- For OpenAI TTS: scene narration text → `api.openai.com`
- For Google Translation: scene narration text → `translation.googleapis.com`
- For ElevenLabs: scene narration text → `api.elevenlabs.io`

API keys are never logged, never embedded in project files, never sent to
any analytics endpoint.

## Fallback chain

When the default voice provider is selected but the Python `edge-tts`
package is not installed, pdf2vid automatically falls back through this
chain on every scene:

1. **edge-tts via Python subprocess** (`pip install edge-tts`)
2. **StreamElements TTS** (English only, public anonymous endpoint)
3. **Google Translate TTS** (all advertised languages, public anonymous endpoint)

The UI never blocks the export on a missing provider — the user always
gets narration audio for every scene. Errors surface in the status bar
and the ProgressModal.