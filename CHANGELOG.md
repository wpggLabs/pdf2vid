# Changelog

All notable changes to pdf2vid are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-06-19

### Added

- Cross-platform Tauri 2 desktop application for Windows, macOS, and Linux
- Real PDF parsing with pdfjs-dist, page thumbnails, per-page scene scripts
- Full-length YouTube (1920×1080) and TikTok (1080×1920) export
- Provider registry with 14 providers across translation, voice, and visual
- edge-tts integration via `python -m edge_tts` subprocess (Microsoft Neural)
- MarianMT local translation (Helsinki-NLP Opus-MT, one-time ~300 MB per pair)
- Piper offline TTS fallback
- OpenAI translation, OpenAI TTS, ElevenLabs TTS, Google Cloud Translation
- Native FFmpeg integration via system PATH or bundled sidecar
- OS credential storage via the `keyring` crate
- Live export progress events with per-stage labels and cancellation
- Debounced project auto-save and on-mount restore
- Streaming PDF import via blob URL with ArrayBuffer fallback
- Tauri dialog-based file picker for large PDF imports
- Preview modal with synthesized TTS playback
- Timeline playback simulation with Skip back/forward and Play
- Tab switching for TIMELINE/SUBTITLES, SCRIPT/SCENE, Scenes/Preview
- Fullscreen toggle and window-focus system status refresh
- Models modal for MarianMT and Piper downloads with progress
- Settings modal with per-category provider and key management
- Cross-platform CI matrix on Windows, macOS, and Ubuntu 22.04
- Release workflow with per-platform tauri-action builds
- FFmpeg sidecar fetch script (`scripts/fetch-ffmpeg.js`)
- 36 unit and integration tests across frontend and backend

### Security

- All API keys stored in the OS credential manager, never in project files
- No telemetry, no analytics, no auto-update pings
- CSP locked down to known origins plus `blob:` for PDF streaming
- License attribution surfaced for every downloadable model

### Known Limitations

- MarianMT local inference (ONNX runtime) is wired through the registry but
  falls back to the original script when the model is not yet supported by
  the bundled inference path
- Piper local inference has the same ONNX runtime gap
- DeepL, Azure Speech, and Higgsfield are honest stubs (Coming soon) with
  the UI disabling selection and exports refusing them

[0.1.0]: https://github.com/wpggLabs/pdf2vid/releases/tag/v0.1.0