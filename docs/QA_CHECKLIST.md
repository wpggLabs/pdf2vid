# Manual QA Checklist

Run before any release. Each step must pass. If a step fails, open a
GitHub issue before merging.

## Setup

- [ ] `npm install` completes without warnings
- [ ] `pip install edge-tts` succeeds
- [ ] `npm run tauri build -- --debug` produces an MSI/EXE
- [ ] `cargo test --lib` passes
- [ ] `npm run test` passes

## Smoke test (automated)

Run `cargo run --example smoke` from `src-tauri/`. The binary:

- [ ] Generates a 3-page PDF fixture
- [ ] Composes both YouTube (1920×1080) and TikTok (1080×1920) outputs
- [ ] ffprobe confirms both files have a video stream and an audio stream
- [ ] Reports bytes saved and exit code

## Import

### Normal PDF (text-based)

- [ ] Click Import PDF, pick a multi-page text PDF
- [ ] Status bar shows "Reading page N of M" with progress
- [ ] Scene list populates with one scene per page
- [ ] Each scene shows a thumbnail and "Page N" label
- [ ] Total duration in the scene-panel footer is reasonable (words/2.5 sec)
- [ ] Project auto-saves (check `app_data_dir/current-project.json` after 1 sec)

### PDF with one blank/no-text page

- [ ] Pick a PDF that has one image-only page in the middle of text pages
- [ ] Import succeeds, that one page is skipped, others import
- [ ] Status bar says "X pages imported · 1 skipped (no text): 5"
- [ ] Scene count equals page count minus one
- [ ] The skipped page number appears in the status string

### Scanned PDF (no text anywhere)

- [ ] Pick a scanned PDF with no OCR
- [ ] Status bar says "No pages had selectable text. Run OCR on this PDF before importing."
- [ ] No scenes added; previous state preserved

## Export

### YouTube only

- [ ] Open Export modal
- [ ] Uncheck TikTok, keep YouTube checked
- [ ] Click Start export
- [ ] Choose a folder
- [ ] Progress modal shows stage transitions: Translating → Synthesizing → Visuals → Composing → Done
- [ ] Final modal shows the YouTube output path
- [ ] File opens in VLC / browser, plays correctly
- [ ] Subtitles visible (drawtext burn-in)
- [ ] No console window flashing during render

### TikTok / Shorts

- [ ] Open Export modal, uncheck YouTube, keep TikTok checked
- [ ] Export completes
- [ ] Output is 1080×1920 (verify with ffprobe or media info)
- [ ] Plays correctly in portrait orientation

### Both formats

- [ ] Both checkboxes enabled
- [ ] Both files appear in the chosen folder
- [ ] Both pass the ffprobe check from the smoke test

## Cancellation

- [ ] Click Export on a longer project (10+ scenes)
- [ ] Hit Cancel during the Composing stage
- [ ] Render aborts within ~1 second (you should see the modal close or show "Cancelled")
- [ ] No partial file is left behind in the output folder (or partial file is clearly marked)
- [ ] App remains responsive, can start a new export immediately

## Preview audio

- [ ] Open the Preview modal (Preview tab in topbar)
- [ ] Click Play on a scene that has TTS available
- [ ] Audio starts within ~2 seconds
- [ ] Click Skip forward rapidly through 5 scenes
- [ ] Audio always matches the visible scene (no stale audio plays for a different image)
- [ ] When Play is pressed before audio loads, button shows "Generating..." and stays disabled
- [ ] When TTS fails (e.g. edge-tts not installed), a clear error appears in the modal

## Provider warnings

### Argos translation + non-English language

- [ ] Set translation provider to Argos, output language to Spanish
- [ ] Open the inspector — offline-translation hint appears
- [ ] With `argostranslate` installed, export translates the scripts
- [ ] Without it, Progress modal shows a warning block: "N scenes used the source script because translation wasn't available", listing page numbers

### Local voice fallback (Kokoro / Chatterbox)

- [ ] Set voice provider to Kokoro (or Chatterbox)
- [ ] With the package missing, export falls back to edge-tts (no hard error)
- [ ] With it installed, the Synthesizing stage shows the first-run
      model-download progress, then produces audio

## Settings

- [ ] Open Settings
- [ ] Category tabs (Translation / Voice / Visual) switch correctly
- [ ] Enter an OpenAI API key, click Save securely
- [ ] Reload the app — key should still be in OS keyring (no re-entry needed)
- [ ] Inspect the OS keyring (Windows: Credential Manager) — entry `com.wpgglabs.pdf2vid/openai` exists

## Dependency checks (no missing tools)

- [ ] Install FFmpeg and verify status bar shows "FFmpeg ready"
- [ ] Unset PATH, restart — status bar should show "FFmpeg not found"
- [ ] Install edge-tts — inspector shows "edge-tts ready"
- [ ] Uninstall edge-tts (`pip uninstall edge-tts`) — inspector shows "edge-tts not detected"
- [ ] In the edge-tts-not-detected state, click Preview voice — clear error mentions `pip install edge-tts`

## Final video validation

For every successfully exported video:

- [ ] File plays in VLC without errors
- [ ] File plays in Chrome (`file://...` in URL bar)
- [ ] Resolution matches expected (1920×1080 or 1080×1920)
- [ ] Audio is present (ffprobe `show_streams` lists an audio stream)
- [ ] Duration equals sum of selected scene durations
- [ ] Subtitles are visible at the bottom of the frame (drawtext burned in)
- [ ] No pixel artifacts at scene boundaries (zoompan transitions look smooth)

## Cross-platform spot check

- [ ] Windows MSI installs cleanly, launches, can complete the smoke workflow
- [ ] macOS DMG installs cleanly (if a Mac build is available)
- [ ] Linux AppImage runs (if a Linux build is available)

## Notes

Add any deviations here before merging:

- Date, build hash, tester
- Steps that produced unexpected results
- Steps skipped due to environment limitations