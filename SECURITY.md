# Security

Report vulnerabilities privately through GitHub Security Advisories for
`wpggLabs/pdf2vid`. Do not open public issues containing credentials or
private PDFs.

## API keys

API keys belong in the operating system credential manager. Project files
and logs must never contain secrets. Online providers must identify what
document data leaves the device before a request is submitted.

## Storage

- **API keys** — OS keyring via the `keyring` crate (Windows Credential
  Manager, macOS Keychain, Linux Secret Service). Keys are scoped per
  provider ID under the `com.wpgglabs.pdf2vid` service name.
- **Project state** — `app_data_dir/current-project.json` (per-platform
  standard location). Created with default user permissions.
- **Model cache** — local voice/translation models (Kokoro, Chatterbox,
  Argos) are managed by their Python packages under the user's cache /
  Hugging Face directories.
- **Audio and video cache** — `app_data_dir/cache/audio/` and
  `app_data_dir/cache/visuals/`. Cleared by the app on each export.

## Network endpoints

The app only contacts the following origins, and only when the
corresponding provider is selected:

- `huggingface.co` — local model weight downloads (Kokoro, Chatterbox).
  One-time per model. Argos language packs come from the Argos index.
- `*.api.cognitive.microsoft.com` — edge-tts synthesis via Python
  subprocess (per scene, when `edge` provider selected).
- `api.streamelements.com` — StreamElements TTS fallback (per scene,
  English only, when edge-tts Python path is unavailable).
- `translate.google.com` — Google Translate TTS fallback (per scene,
  when edge-tts Python path is unavailable).
- `api.openai.com` — OpenAI translation and TTS (per scene, when OpenAI
  selected).
- `translation.googleapis.com` — Google Cloud Translation (per scene,
  when Google selected).
- `api.elevenlabs.io` — ElevenLabs TTS (per scene, when ElevenLabs selected).
- `pypi.org` — OCR engine install. When a scanned/image-only page is
  imported, the app lazily creates a **dedicated Python venv** under the
  app data directory and `pip install`s `rapidocr-onnxruntime` +
  `pillow` (one-time; PyPI + its CDN). No system Python is modified,
  and no code runs outside that venv. The OCR subprocess is invoked with
  the image path passed as an argv argument (no shell interpolation), so
  the page image cannot inject Python.

No telemetry, no analytics, no auto-update pings.

## Local model licenses

The bundled local providers are permissively licensed: Kokoro (Apache-2.0),
Chatterbox (MIT), and Argos Translate (MIT). They are opt-in Python
packages the user installs explicitly.

## Disabling network entirely

To run the app with zero network access after the initial model download,
choose a local voice provider (**Kokoro** or **Chatterbox**) with **Argos**
translation. All three run fully offline once their weights/packs are cached.

`edge-tts` is the highest-quality free default but requires Python with
the `edge-tts` package installed plus network access to Microsoft's
endpoint at synthesis time. The UI tags it as "Microsoft Neural via
Python" so the choice is honest.