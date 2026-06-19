# Contributing

Contributions are welcome. Open an issue before large architectural changes.

## Ground Rules

1. Keep providers behind the provider registry in
   `src-tauri/src/providers.rs`. Add new providers via a `ProviderOption`
   entry and wire a concrete implementation.
2. Never log API keys, PDF contents, or generated narration.
3. Run the frontend build, frontend tests, and Rust tests before
   submitting.
4. Keep pull requests focused and document user-visible behavior.

## Provider Rules

- **Paid providers must never be selected automatically over a free local
  provider.** The default selection is the first item in each provider
  list, which is always the free option. Tests in
  `src-tauri/src/providers.rs` enforce this invariant.
- Local providers should run offline after first-use model download
  (MarianMT, Piper).
- Cloud providers must accept the user's API key from the OS keyring —
  never accept it through another channel.
- New "Coming soon" providers should be added to the registry with
  `implemented: false` so the UI disables them and never offers them as
  a real export option.

## Architecture Pointers

- `src-tauri/src/render.rs` — export pipeline. Stages: plan → translate →
  synthesize → visual → compose. Each stage emits progress events.
- `src-tauri/src/providers.rs` — provider registry, free defaults, and
  edge-voice → language mapping.
- `src-tauri/src/models.rs` — MarianMT and Piper model registry,
  download with progress events, license attribution.
- `src-tauri/src/edgetts.rs` — edge-tts integration (Python subprocess).
- `src-tauri/src/cloud.rs` — OpenAI, Google, ElevenLabs HTTP clients,
  plus the MarianMT/Piper ONNX integration points.
- `src-tauri/src/ffmpeg.rs` — FFmpeg detection, sidecar lookup, aspect
  ratio helpers.
- `src/components/ProgressModal.tsx` — UI for live progress and
  cancellation.
- `docs/ARCHITECTURE.md` — full backend/frontend data flow.

## Local Model License Attribution

When adding a new model entry to `models.rs`, set the `license` field to
the model's actual license string (e.g. `CC-BY-4.0`, `CC-BY-NC-4.0`,
`Apache-2.0`). The UI surfaces this in the model picker; non-commercial
licenses trigger an explicit acceptance prompt before download.

## Development Setup

```bash
git clone https://github.com/wpggLabs/pdf2vid
cd pdf2vid
npm install
pip install edge-tts
npm run tauri dev
```

## Validation

Before opening a pull request, run the full preflight:

```bash
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run tauri build -- --debug
```

All four must succeed. CI will run the same checks across Windows, macOS,
and Ubuntu 22.04.