# Contributing

1. Open an issue before large architectural changes.
2. Keep providers behind the provider registry in `src-tauri/src/providers.rs`.
   The trait-style registry lives in the same module; new providers should add a
   `ProviderOption` entry and wire a concrete implementation.
3. Never log API keys, PDF contents, or generated narration.
4. Run the frontend build, frontend tests, and Rust tests before submitting.
5. Keep pull requests focused and document user-visible behavior.

## Provider rules

- **Paid providers must never be selected automatically over a free local provider.**
  The default selection is the first item in each provider list, which is always the
  free option. Tests in `src-tauri/src/providers.rs` enforce this invariant.
- Local providers should run offline after first-use model download (MarianMT, Piper).
- Cloud providers must accept the user's API key from the OS keyring — never accept
  it through another channel.
- New "Coming soon" providers should be added to the registry with `implemented:
  false` so the UI disables them and never offers them as a real export option.

## Architecture pointers

- `src-tauri/src/render.rs` — the export pipeline. Stages: plan → translate →
  synthesize → visual → compose. Each stage emits progress events.
- `src-tauri/src/providers.rs` — provider registry, free defaults, and edge-voice
  → language mapping.
- `src-tauri/src/models.rs` — MarianMT and Piper model registry, download with
  progress events, license attribution.
- `src-tauri/src/edgetts.rs` — edge-tts integration point.
- `src-tauri/src/cloud.rs` — OpenAI, Google, ElevenLabs HTTP clients, plus the
  MarianMT/Piper ONNX integration points.
- `src-tauri/src/ffmpeg.rs` — FFmpeg detection, sidecar lookup, aspect ratio helpers.
- `src/components/ProgressModal.tsx` — UI for live progress and cancellation.

## Local model license attribution

When adding a new model entry to `models.rs`, set the `license` field to the model's
actual license string (e.g. `CC-BY-4.0`, `CC-BY-NC-4.0`, `Apache-2.0`). The UI
surfaces this in the model picker; non-commercial licenses trigger an explicit
acceptance prompt before download.