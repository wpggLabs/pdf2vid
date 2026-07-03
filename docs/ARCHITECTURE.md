# Architecture

This document describes the runtime architecture of pdf2vid so contributors
and tooling can navigate the codebase.

## High-Level Shape

```
┌─────────────────────────────────────────────────────────────────┐
│  WebView (Tauri 2)                                              │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  React 19 + TypeScript UI                                 │  │
│  │  - App.tsx: shell, state, layout                          │  │
│  │  - components/: ExportModal, SettingsModal, ModelsModal,  │  │
│  │    ProgressModal, PreviewModal                             │  │
│  │  - hooks/: useTimelinePlayback, usePreviewVoice,          │  │
│  │    useTranslationModelPrompt                              │  │
│  │  - pdf.ts: pdfjs-dist streaming import                    │  │
│  │  - backend.ts: typed invoke() + listen() wrappers         │  │
│  └───────────────────────────────────────────────────────────┘  │
│            │ invoke / listen (Tauri IPC)                         │
└────────────┼─────────────────────────────────────────────────────┘
             │
┌────────────▼─────────────────────────────────────────────────────┐
│  Rust process (Tauri commands + plugins)                         │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  commands.rs: 14 #[tauri::command] handlers               │  │
│  │  state.rs:    AppState with active_job + cancel flag       │  │
│  └───────────────────────────────────────────────────────────┘  │
│            │                                                    │
│  ┌─────────▼────────┐  ┌────────────────┐  ┌─────────────────┐  │
│  │  render.rs       │  │  edgetts.rs    │  │  cloud.rs       │  │
│  │  4-stage pipeline│  │  Python edge-  │  │  OpenAI, Google │  │
│  │  plan→translate  │  │  tts subprocess│  │  ElevenLabs HTTP│  │
│  │  →synthesize     │  └────────────────┘  └─────────────────┘  │
│  │  →visual→compose │                                           │
│  └──────────────────┘  ┌────────────────┐  ┌─────────────────┐  │
│                        │  kokoro/chatter│  │  ffmpeg.rs      │  │
│                        │  box/argos +   │  │  detection +    │  │
│                        │  subprocess.rs │  │  arg builders   │  │
│                        └────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
             │
┌────────────▼─────────────────────────────────────────────────────┐
│  System                                                            │
│  - FFmpeg (PATH or bundled sidecar)                              │
│  - Python + edge-tts (Microsoft Neural voices)                    │
│  - Hugging Face (model downloads)                                │
│  - Provider HTTP APIs (OpenAI, Google, ElevenLabs)               │
│  - OS Credential Manager (keyring)                               │
└─────────────────────────────────────────────────────────────────┘
```

## Frontend Layer

### State Management

`App.tsx` is the single source of UI truth. It holds:

- `project: Project` — the full project model (scenes, providers, language)
- `providers: ProviderList | null` — fetched once on mount via `list_providers`
- `system: SystemStatus` — FFmpeg/FFprobe availability, refreshed on `focus`
- `activeId: string` — the currently selected scene
- `aspect: "youtube" | "tiktok"` — preview aspect ratio
- `workspaceTab`, `timelineTab`, `inspectorTab` — UI navigation state
- `settingsOpen`, `modelsOpen`, `exportOpen`, `progressJobId`, `previewOpen` — modal state

Persistence: project state is auto-saved to Rust via `save_project` on every
change (debounced 600 ms) and restored on mount via `load_project`.

### Components

| Component | Purpose |
|---|---|
| `App.tsx` | Three-pane shell, status bar, top-level modals |
| `ExportModal` | Output folder picker via `@tauri-apps/plugin-dialog` |
| `SettingsModal` | Per-category provider and API key configuration |
| `ModelsModal` | Local model download/delete with progress |
| `ProgressModal` | Live export progress with stage labels and cancel |
| `PreviewModal` | Fullscreen scene preview with TTS playback |

### Hooks

| Hook | Responsibility |
|---|---|
| `useTimelinePlayback` | Timer-driven scene advancement for the Play button |
| `usePreviewVoice` | Manages a single `<audio>` element and TTS fetch |
| `useTranslationModelPrompt` | Legacy MarianMT model prompt (Argos now auto-installs packs) |

### Backend Bridge (`backend.ts`)

Typed wrappers around `invoke()` and `listen()` for every Tauri command and
event. Components and hooks only import from this module — they never call
`invoke` directly.

## Rust Backend Layer

### Command Surface

All Tauri commands are registered in `lib.rs:21` and implemented in
`commands.rs`:

| Command | Purpose |
|---|---|
| `system_status` | FFmpeg/FFprobe/platform availability |
| `save_project` / `load_project` | Project persistence in `app_data_dir` |
| `store_api_key` | Keyring-backed secret storage |
| `list_providers` | Returns the provider registry descriptor |
| `list_models` / `download_model` / `delete_model` / `is_model_installed` | Local model management |
| `translate_text` | One-shot translation preview |
| `preview_voice` | One-shot TTS preview (returns data URL) |
| `validate_export` | Pre-flight check (output formats, scenes, scripts, FFmpeg) |
| `start_export` | Kicks off the 4-stage render pipeline |
| `cancel_export` | Flips the active job's cancel flag |
| `read_pdf_file` | Rust-side PDF read for large file imports |
| `check_tts_engine` | Detects Python + edge-tts availability |

### State (`state.rs`)

```rust
pub struct AppState {
    pub active_job: Mutex<Option<JobHandle>>,
}

pub struct JobHandle {
    pub job_id: String,
    pub cancel_flag: Arc<AtomicBool>,
}
```

Only one export runs at a time. `start_export` registers a handle; the
render pipeline checks `cancel_flag` between stages and after FFmpeg
invocations.

### Provider Registry (`providers.rs`)

Single source of truth for the provider list. Each entry carries:
`id`, `label`, `kind` (local/api), `detail`, `implemented`, `online`,
`key_label`, `category` (translation/voice/visual).

Tests enforce:
- First entry of each category is local and implemented (free default)
- IDs are unique within each category
- Stub providers are marked `implemented: false`

### Render Pipeline (`render.rs`)

`run_export(app, state, request)` orchestrates four stages, each emitting
`export:progress` events:

1. **Plan** — validate output formats, scenes, scripts, FFmpeg
2. **Translate** — Argos (local subprocess) or cloud HTTP per scene
3. **Synthesize** — voice provider with automatic fallback chain; edge-tts
   also emits subtitle cues for word-accurate read-along captions
4. **Visual + Compose** — write page JPGs, run FFmpeg with the premium
   filtergraph (blurred backdrop, Ken Burns, vignette, timed captions)

The pipeline is async-aware but uses `std::process::Command` for FFmpeg
because FFmpeg's output is naturally bounded (one MP4 per scene-pair, not a
stream we need to drain mid-process). The Rust standard library handles
process lifecycle correctly within an async context.

### TTS Backend Selection (`edgetts.rs`, `cloud.rs`)

Voice synthesis order of preference:

1. **`edge` provider** — spawns `python -m edge_tts --text ... --voice ... --write-media <tmp>` and reads the resulting MP3. Best free quality (Microsoft Neural). Requires Python 3.8+ and the `edge-tts` package.
2. **StreamElements** — public anonymous endpoint, English-only, Amazon Polly voices under the hood. No auth.
3. **Google Translate TTS** — public anonymous endpoint, all advertised languages, lower quality.

`render.rs::synthesize_scene_audio` implements a fallback chain: it tries
the user's chosen provider first, then falls back to the next free option
on transient errors. Only hard errors (missing API key, missing model,
unimplemented stub) are surfaced immediately.

### FFmpeg Detection (`ffmpeg.rs`)

Resolution order:
1. Sidecar binary next to the application executable (`ffmpeg.exe` / `ffmpeg`)
2. `which ffmpeg` against the system `PATH`

If neither is found, `ensure_ffmpeg_or_error()` returns a clear error
message that the frontend surfaces in the status bar and the ProgressModal.

### Model Registry (`models.rs`)

Models are described by a static `ModelSpec` table. Note that the
default local providers (Argos, Kokoro, Chatterbox) manage their own
weights via their Python packages and stream download progress through
`subprocess::run_with_progress`, so they do not need entries here.

`download_model` streams each file with progress events:
`model:progress { downloaded, total, percent }` and a final
`model:complete { success }`. SHA-256 verification is supported but
disabled by default because the model files are trusted sources
(`huggingface.co` over HTTPS).

## Data Flow: PDF to MP4

```
User clicks "Import PDF"
  ↓
App.tsx: pickAndImportPdf() → openDialog (Tauri)
  ↓
parsePdf({ kind: "path", path })
  ↓
backend.readPdfFile(path) → Vec<u8>
  ↓
pdfjs.getDocument({ data: Uint8Array }).promise
  ↓
Per page: getTextContent + getViewport + canvas render
  ↓
Scene[] with extracted text and JPEG thumbnails
  ↓
setProject({ ...current, scenes }) → auto-save (debounced)

User clicks "Export"
  ↓
ExportModal: handleStart() → saveDialog + startExport(jobId, project, dir)
  ↓
App.tsx: backend.startExport → commands::start_export
  ↓
render::run_export
  ├─ emit "Planning"
  ├─ emit "Translating" (cloud or marian)
  ├─ emit "Synthesizing" (edge-tts → fallback)
  ├─ emit "Visuals" (write JPGs)
  ├─ emit "Composing" (FFmpeg filtergraph)
  └─ emit "Done" with output paths

ProgressModal listens for export:progress / export:done / export:error
  ↓
User sees per-stage updates and the Cancel button
```

## Tauri IPC Schema

### Frontend → Backend (`invoke`)

All payloads are camelCase on the wire. Rust deserializes with
`#[serde(rename_all = "camelCase")]` on each struct.

### Backend → Frontend (`emit`)

Events:
- `export:progress` — `ExportProgress`
- `export:done` — `ExportComplete`
- `export:error` — `ExportError`
- `model:progress` — `ModelDownloadProgress`
- `model:complete` — `{ modelId, success }`

All event payloads are typed in `src/api.ts` and matched by the
`on*` listeners in `backend.ts`.

## Concurrency Model

- **Tauri commands** marked `async` run on the tokio runtime
- **`std::process::Command`** for FFmpeg is synchronous and runs on the
  blocking thread; the Rust command awaits the result
- **State mutex** is `tokio::sync::Mutex`, never `std::sync::Mutex`
- **Cancel flag** is `AtomicBool` for lock-free check between stages

## Build Artifacts

| Platform | Artifact | Type |
|---|---|---|
| Windows | `pdf2vid_0.1.0_x64_en-US.msi` | WiX MSI |
| Windows | `pdf2vid_0.1.0_x64-setup.exe` | NSIS EXE |
| macOS | `pdf2vid_0.1.0_*.dmg` | DMG |
| macOS | `pdf2vid.app` | Bundle |
| Linux | `pdf2vid_0.1.0_amd64.deb` | Debian |
| Linux | `pdf2vid_0.1.0_amd64.AppImage` | AppImage |
| Linux | `pdf2vid-0.1.0-*.rpm` | RPM |

See [BUILDING.md](../BUILDING.md) for build commands per platform.