# pdf2vid

`pdf2vid` is a local-first desktop studio for turning text-based PDFs into editable,
narrated video projects for YouTube and TikTok.

## Current release

- Cross-platform Tauri 2 application for Windows, macOS, and Linux.
- Real PDF parsing, page thumbnails, page selection, and editable scene scripts.
- Full-length 1920x1080 YouTube and 1080x1920 TikTok export configuration.
- Free local provider defaults with optional bring-your-own API integrations.
- Provider registry for Argos Translate, Piper, DeepL, Google Cloud, OpenAI,
  Azure Speech, ElevenLabs, and Higgsfield.
- Native FFmpeg diagnostics, local project persistence, and OS credential storage.
- No account requirement and no project telemetry.

> The editor and project pipeline are functional. The renderer currently validates
> projects and native dependencies; bundled Piper/Argos model download and final
> FFmpeg timeline rendering are the next release milestone. The app does not claim
> to render completed narration until those components are installed.

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

## Privacy and API keys

Local providers keep PDF text on the device. API providers transmit only the data
needed for the selected operation. Desktop API keys are stored in the operating
system credential manager and are never written to project files.

Do not use sensitive documents with an online provider unless its privacy terms are
acceptable for the document.

Issues and releases: [wpggLabs/pdf2vid](https://github.com/wpggLabs/pdf2vid)

## License

MIT
