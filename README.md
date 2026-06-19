# pdf2vid

<p align="center">
  <img src="assets/banner.svg" alt="pdf2vid" width="100%">
</p>

<p align="center">
  <strong>Local-first desktop studio that turns text-based PDFs into narrated
  video projects for YouTube and TikTok.</strong>
</p>

<p align="center">
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <a href="https://v2.tauri.app/"><img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=black"></a>
  <a href="https://react.dev/"><img alt="React" src="https://img.shields.io/badge/React-19-149ECA?logo=react&logoColor=white"></a>
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=black"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/Platforms-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey">
</p>

<p align="center">
  <a href="#installation">Installation</a> &middot;
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="#providers">Providers</a> &middot;
  <a href="docs/ARCHITECTURE.md">Architecture</a> &middot;
  <a href="docs/providers.md">Provider Reference</a> &middot;
  <a href="BUILDING.md">Building</a>
</p>

---

<p align="center">
  <img src="assets/screenshot.png" alt="pdf2vid editor at 1440x1024" width="900">
</p>

## Highlights

- **Real PDF parsing** — text extraction, page thumbnails, scene selection, editable per-scene scripts
- **YouTube 1920×1080** and **TikTok 1080×1920** full-length export
- **Free by default** — works with zero accounts, zero API keys, zero subscriptions
- **Optional paid upgrades** — bring your own OpenAI, Google Cloud, or ElevenLabs key for higher quality
- **Provider registry** — OpenAI, Google Cloud Translation, ElevenLabs, edge-tts, Piper, MarianMT
- **Native FFmpeg integration** — system PATH or bundled sidecar
- **OS credential storage** — keys never written to project files
- **Live progress** with cancellation, debounced auto-save, and streaming PDF import
- **Cross-platform** — Windows 10+, macOS 11+, Ubuntu 22.04+

## Installation

### Windows

Download the latest `pdf2vid_0.1.0_x64-setup.exe` from the
[Releases](../../releases) page. The MSI and NSIS installers bundle FFmpeg
auto-detection and the desktop runtime.

```powershell
winget install pdf2vid
# or download from the Releases page
```

### macOS / Linux

See [BUILDING.md](BUILDING.md) for the build matrix or download the
appropriate `.dmg` / `.deb` / `.AppImage` / `.rpm` from the Releases page.

### Optional: edge-tts for the highest-quality free voice

The default voice provider (`edge-tts`) shells out to the Python `edge-tts`
package, which calls the free Microsoft Edge browser TTS endpoint. Install
it once:

```bash
pip install edge-tts
```

If `edge-tts` is unavailable, pdf2vid automatically falls back to
StreamElements (English) and Google Translate TTS (other languages). No
crash, no error.

## Quick Start

1. **Launch pdf2vid.** The status bar should read "Ready".
2. **Click Import PDF** in the top-left, or drag a PDF onto the window.
   Text-based PDFs work directly; scanned PDFs need OCR first.
3. **Pick the output language** and providers in the right inspector.
   Free defaults work without setup.
4. **Edit scene scripts** in the bottom editor. Each PDF page becomes
   one scene.
5. **Click Export video**, choose a folder, watch the progress bar.
   Cancel any time.

For higher quality, open **Settings**, paste an OpenAI or ElevenLabs API
key, and pick the cloud provider from the dropdown.

## Providers

pdf2vid exposes every stage through a single registry. The user picks one
provider per stage. The first option in each list is the free default.

| Stage | Free default | Optional paid | Notes |
|---|---|---|---|
| Translation | MarianMT (local, CC-BY-4.0) | OpenAI, Google Cloud | Model download ~300 MB per pair |
| Voice | edge-tts (Microsoft Neural via Python) | OpenAI TTS, ElevenLabs | Requires `pip install edge-tts` |
| Visual | PDF pages + Ken Burns + drawtext | Higgsfield (coming soon) | Zero dependencies |
| Render | FFmpeg (system PATH or sidecar) | — | Bundled by default |

See [docs/providers.md](docs/providers.md) for the full per-provider
matrix with data flow, license, and network behavior.

## Repository Layout

```
pdf2vid/
├── assets/                  # README banner, screenshot
├── docs/
│   ├── ARCHITECTURE.md      # Backend/frontend architecture
│   ├── providers.md        # Provider reference
│   └── design/              # UI design references
├── src/                     # React + TypeScript frontend
│   ├── App.tsx              # Top-level component
│   ├── pdf.ts               # pdfjs-dist streaming import
│   ├── backend.ts           # Typed Tauri command/event wrappers
│   ├── components/          # Modal and view components
│   └── hooks/               # Timeline playback, preview voice, etc.
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── lib.rs           # Tauri builder + command registration
│   │   ├── commands.rs      # 14 #[tauri::command] handlers
│   │   ├── render.rs        # 4-stage export pipeline
│   │   ├── providers.rs     # Provider registry
│   │   ├── models.rs        # MarianMT + Piper model registry
│   │   ├── edgetts.rs       # edge-tts Python subprocess
│   │   ├── cloud.rs         # OpenAI, Google, ElevenLabs HTTP clients
│   │   ├── ffmpeg.rs        # FFmpeg detection + arg builders
│   │   └── state.rs         # AppState, JobHandle, cancel flag
│   ├── tauri.conf.json
│   ├── capabilities/
│   └── Cargo.toml
├── scripts/
│   └── fetch-ffmpeg.js      # Platform-specific FFmpeg downloader
├── BUILDING.md              # Build matrix per platform
├── CONTRIBUTING.md
├── SECURITY.md
└── LICENSE
```

## Development

Requirements: Node.js 20+, Rust stable, the
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/), FFmpeg,
and FFprobe.

```bash
git clone https://github.com/wpggLabs/pdf2vid
cd pdf2vid
npm install
npm run tauri dev          # Development build with HMR
```

Web-only UI development (without the Rust shell):

```bash
npm run dev
```

Validation:

```bash
npm run test                 # Vitest (frontend)
npm run build                # TypeScript + Vite production build
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run tauri build -- --debug
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The non-negotiable rules:

1. Keep providers behind the provider registry in `src-tauri/src/providers.rs`
2. Never log API keys, PDF contents, or generated narration
3. Paid providers must never be selected automatically over a free local provider
4. Run the full test suite before submitting

## Security

Report vulnerabilities through GitHub Security Advisories. See
[SECURITY.md](SECURITY.md) for storage, network endpoints, and license gates.

## License

MIT — see [LICENSE](LICENSE).

---

<p align="center">
  <sub>Built with Tauri 2, React 19, Rust, pdfjs-dist, FFmpeg, and the
  <a href="https://github.com/rany2/edge-tts">edge-tts</a> Python package.</sub>
</p>