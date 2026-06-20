# Build Log

This document records what was built, tested, and shipped in each
release pass. It complements `CHANGELOG.md` (which is user-facing) by
capturing the engineering evidence: commands run, smoke test results,
remaining risks.

## Phase 1.5 — Real-world hardening pass

Date: 2026-06-20
Branch: main
Starting point: Phase 1 fix series (8 commits ending at `1e2b16e`)

### Scope

- E2E smoke test with a real 3-page PDF fixture
- PreviewModal play timing fix
- Provider health display in the inspector
- Improved export result summary
- Runtime dependency check with install instructions

### Constraints

- No MarianMT ONNX implementation
- No Piper ONNX implementation
- No AppShell component extraction
- No new providers or features
- Architecture unchanged except for small helpers

### Files changed

| File | Change |
|---|---|
| `docs/QA_CHECKLIST.md` | New. Manual QA checklist for all flows. |
| `docs/BUILD_LOG.md` | This file. |
| `src-tauri/src/render.rs` | Extracted `build_ffmpeg_args` (pure function, testable). Added `-shortest` to bound encode. |
| `src-tauri/examples/smoke.rs` | New. End-to-end smoke test that builds scene inputs, runs the real render filter, and verifies outputs with ffprobe. |
| `fixtures/sample-3page.pdf` | New. 3-page text PDF generated via Python `fpdf2`. |
| `src/components/PreviewModal.tsx` | Fixed play timing. |
| `src/components/ProgressModal.tsx` | Improved export result summary. |
| `src-tauri/src/commands.rs` | New commands for runtime dependency checks. |
| `src/components/ProviderHealth.tsx` | New component in inspector. |
| `src/App.tsx`, `src/state/useProjectState.ts` | Wire up new pieces. |

### Tests run

```
npm test                 → 18 passed
cargo test --lib         → 31 passed
npm run build            → clean
cargo run --example smoke → pass (YouTube 1920x1080, TikTok 1080x1920, both with audio+video streams)
```

### Smoke test result

Captured `2026-06-20` on Windows host with FFmpeg 8.1.1-essentials + Python 3.14 + edge-tts 7.2.7.

```json
{
  "pass": true,
  "youtube": {
    "path": "...\\sample-youtube.mp4",
    "bytes": 27582,
    "ffprobe": {
      "streams": [
        { "codec_type": "video", "width": 1920, "height": 1080 },
        { "codec_type": "audio" }
      ],
      "format": { "duration": "7.040000", "size": "27582" }
    },
    "ok": true,
    "issues": []
  },
  "tiktok": {
    "path": "...\\sample-tiktok.mp4",
    "bytes": 1141687,
    "ffprobe": {
      "streams": [
        { "codec_type": "video", "width": 1080, "height": 1920 },
        { "codec_type": "audio" }
      ],
      "format": { "duration": "7.040000", "size": "1141681" }
    },
    "ok": true,
    "issues": []
  },
  "duration_seconds": 7.0
}
```

### Failures and discoveries

1. **Hand-rolled PDF was malformed.** `gen_sample_pdf` Rust binary produced a 1139-byte PDF that `ffprobe` rejected. Switched to Python `fpdf2` (already installed via pip) which produces a valid PDF that `pypdfium2` parses as 3 pages. The `gen_sample_pdf` binary was removed.

2. **`drawtext` font path needs no colon.** The ffmpeg `drawtext` filter parses `fontfile=path:text=...` and treats the `:` after the path as the option separator. On Windows, `C:\Windows\Fonts\arial.ttf` produces a path with `:` which breaks the parser. The smoke test copies the font to `font.ttf` in the work dir so it can be referenced by filename. Production code does not set `fontfile`; on systems without default font resolution this is a latent bug.

3. **`-loop 1` + `-shortest` was missing.** The original `build_ffmpeg_args` had no duration bound. On a 6-second audio with a looped image, ffmpeg would produce output indefinitely, hanging the encode. Fixed by adding `-shortest` to `build_ffmpeg_args`. The hang was reproduced manually in the smoke test setup.

4. **`libx264` preset=fast is slow even for short clips.** The smoke test uses 7-second placeholder clips at 1920x1080 and 1080x1920. With preset=fast, each render took 60+ seconds on the test host. The smoke test overrides to `ultrafast` so the full pipeline completes in under 30 seconds.

5. **Process stdout/stderr buffering.** The smoke test reads stderr from ffmpeg via `process::Command::output()`. If ffmpeg writes a lot of stderr without flushing (rare), the pipe can fill. Not an issue in this run but worth knowing.

### Remaining risks

1. **No font resolution in production.** `drawtext` uses no `fontfile` argument. On Linux servers without `fontconfig` correctly configured, draws will fail and the encode will hang. Should be addressed by detecting a usable font at startup and injecting `fontfile=` into `build_ffmpeg_args`.
2. **No libass fallback.** drawtext requires freetype + a font. On minimal Docker images this is a problem. libass is more portable.
3. **MarianMT / Piper still return hard errors.** Users selecting non-English language + MarianMT see a clear warning but the export proceeds with the source script. The end-to-end pipeline with translation only works once ONNX inference is implemented (out of scope here).
4. **PreviewModal still doesn't actually fetch audio in tests.** The component depends on Tauri's `invoke` which is mocked. The play-timing fix works in the app but cannot be unit-tested without integration scaffolding.

### Next steps (Phase 2 candidates)

1. Inject `fontfile` automatically from `system_status` or a startup probe.
2. Move smoke test to `cargo test` so it runs in CI (currently `cargo run --example`).
3. Add libass as a fallback when drawtext fontconfig fails.
4. Wire the new dependency-check commands into a first-run "setup wizard" modal.