# Security

Report vulnerabilities privately through GitHub Security Advisories for
`wpggLabs/pdf2vid`. Do not open public issues containing credentials or private PDFs.

## API keys

API keys belong in the operating system credential manager. Project files and logs
must never contain secrets. Online providers must identify what document data leaves
the device before a request is submitted.

## Storage

- API keys: OS keyring via the `keyring` crate (Windows Credential Manager, macOS
  Keychain, Linux Secret Service).
- Project state: `app_data_dir/current-project.json` (per-platform standard
  location). Created with default user permissions.
- Model cache: `app_data_dir/models/<model-id>/` for MarianMT and Piper. Contains
  `.installed` marker file plus downloaded model files.
- Audio and video cache: `app_data_dir/cache/audio/` and `app_data_dir/cache/visuals/`.
  Cleared by the app on each export.

## Network endpoints

The app only contacts the following origins, and only when the corresponding
provider is selected:

- `huggingface.co` — model file downloads (MarianMT, Piper voices). One-time per model.
- `*.api.cognitive.microsoft.com` — edge-tts synthesis (per scene).
- `api.openai.com` — OpenAI translation and TTS (per scene, when OpenAI selected).
- `translation.googleapis.com` — Google Cloud Translation (per scene, when Google selected).
- `api.elevenlabs.io` — ElevenLabs TTS (per scene, when ElevenLabs selected).

No telemetry, no analytics, no auto-update pings.

## Local model license gates

Some MarianMT and Piper voices carry non-permissive licenses (CC-BY-NC). The model
picker surfaces the license before download and requires explicit user acceptance
for non-commercial-restricted models.

## Disabling network entirely

To run the app with zero network access after the initial model download, choose
the **Piper** voice provider and the **MarianMT** translation provider. Both run
fully offline once their model files are cached.

edge-tts is the highest-quality free default but requires Python with the
`edge-tts` package installed plus network access to Microsoft's endpoint at
synthesis time. The UI tags it as "Microsoft Neural via Python" so the choice
is honest.