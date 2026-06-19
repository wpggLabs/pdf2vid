# Design QA

- Reference: `docs/design/pdf2vid-editor-reference.png`
- Implementation: `docs/design/pdf2vid-editor-implementation.png`
- Viewport: 1440x1024
- Capture: automated Chromium review at `http://127.0.0.1:1420`

## Comparison

| Area | Result |
| --- | --- |
| Layout | Passed. Three-pane editor, top command bar, central preview, bottom timeline, and status bar match the reference hierarchy. |
| Typography | Passed. Compact neutral UI face and monospace time data preserve the reference density and hierarchy. |
| Palette | Passed. Graphite surfaces, restrained blue selection, green local status, and subtle dividers match the reference system. |
| Controls | Passed. Provider selectors, scene selection, transport, script editor, export formats, and primary export action are present and functional. |
| Timeline | Passed. Scene clips, waveform, subtitle blocks, time ruler, and selected state match the reference anatomy. |
| Spacing | Passed. Panel gutters, compact rows, toolbar heights, and inspector rhythm stay consistent at the native viewport. |
| Icons | Passed. Phosphor icons provide a consistent professional stroke/fill family without custom approximations. |

## Copy Diff

Navigation and workflow labels match the reference intent. The implementation uses an
honest empty-project state instead of the reference's fictional clean-energy PDF. This
is intentional product behavior, not a fidelity mismatch; imported PDFs populate the
same scene and preview structures.

## Interaction QA

- Settings opens the provider dialog with category tabs.
- Models modal lists MarianMT and Piper downloads with size, license, progress, and delete.
- Export modal shows output folder picker via the dialog plugin.
- Progress modal streams live stage and percent events from the Rust pipeline and offers Cancel.
- API key input accepts masked values, saves into OS keyring, never into project files.
- Provider, language, voice, aspect-ratio, page-selection, playback, and script controls expose functional state.
- Stub providers are visually disabled with "Coming soon" badge.
- Browser console contains no errors or warnings.

Final result: passed