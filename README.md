# pdf2vid

`pdf2vid` is a local-first desktop studio for turning text-based PDFs into editable,
narrated video projects for YouTube and TikTok.

## Current release

- Cross-platform Tauri 2 application for Windows, macOS, and Linux.
- Real PDF parsing, page thumbnails, page selection, and editable scene scripts.
- Full-length 1920x1080 YouTube and 1080x1920 TikTok export configuration.
- **Free by default**: works with no account and no API keys.
- **Optional paid upgrades**: bring your own OpenAI, ElevenLabs, or Google Cloud API key for higher quality translation or voice.
- Native FFmpeg integration via system PATH or bundled sidecar.
- Local project persistence in `app_data_dir`, OS credential storage for API keys.
- Live progress events with cancellation.
- No account requirement and no project telemetry.

## Free vs paid path

Every PDF → video export works with **zero cost** using the local default providers:

| Stage | Free default | Optional paid upgrade |
|---|---|---|
| Translation | MarianMT (Helsinki-NLP, runs locally after model download) | OpenAI, Google Cloud Translation |
| Voice | edge-tts (Microsoft Neural, free, requires network at synthesis) | OpenAI TTS, ElevenLabs |
| Visual | PDF page thumbnails with Ken Burns + subtitle burn | — |
| Render | FFmpeg (system PATH or bundled sidecar) | — |

See [`docs/providers.md`](docs/providers.md) for full per-provider details.

## Network usage

- **edge-tts**: each scene narration is sent to `*.api.cognitive.microsoft.com`. The audio bytes come back as MP3. No login required.
- **MarianMT model download**: one-time per language pair from `huggingface.co` (~300 MB per pair). After that, fully offline.
- **Paid providers**: your API key is used directly from your machine against the provider's API. Keys are stored in your OS credential store and never written to project files.

## Development

Requirements: Node.js 20+, Rust stable, the
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/), FFmpeg, and FFprobe.

```bash
npm install
npm run tauri dev
```

Web-only UI development: `npm run dev`.

Validation:

```bash
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug
```

## First-run quick start

1. Click **Import PDF** and select a text-based PDF (scanned/image PDFs need OCR first).
2. Pick the output language and providers — the free defaults work without any setup.
3. Edit scene scripts in the right inspector.
4. Click **Export video** and choose a folder.
5. Watch the progress bar; cancel any time.

For higher quality, open **Settings**, paste an OpenAI or ElevenLabs API key, and pick the cloud provider from the dropdown.

## Privacy and API keys

Local providers keep PDF text on the device. API providers transmit only the data
needed for the selected operation. Desktop API keys are stored in the operating
system credential manager and are never written to project files.

Do not use sensitive documents with an online provider unless its privacy terms are
acceptable for the document.

Issues and releases: [wpggLabs/pdf2vid](https://github.com/wpggLabs/pdf2vid)

## License

MIT