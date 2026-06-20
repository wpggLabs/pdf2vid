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

- E2E smoke test with a real 3-page PDF
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

(populated below as work completes)