# Building pdf2vid

This document covers building installable artifacts for Windows, macOS,
and Linux from source.

## Prerequisites

- **Node.js** 20 or later
- **Rust** stable toolchain (`rustup default stable`)
- **Platform Tauri prerequisites** — see
  https://v2.tauri.app/start/prerequisites/
- **Linux extra packages** — `libwebkit2gtk-4.1-dev libappindicator3-dev
  librsvg2-dev patchelf`
- **macOS** — Xcode Command Line Tools (`xcode-select --install`)
- **Windows** — WebView2 runtime (preinstalled on Windows 10+)
- **FFmpeg** — installed system-wide, or bundled via the sidecar script
- **Python 3.8+** with `edge-tts` for the default voice provider

## Quick Start

```bash
git clone https://github.com/wpggLabs/pdf2vid
cd pdf2vid
npm install
pip install edge-tts          # for the default voice provider
npm run tauri build           # produces installers for the current platform
```

## Platform-Specific Builds

```bash
npm run build:win      # Windows: .msi + .exe
npm run build:mac      # macOS:   .dmg + .app (Intel + Apple Silicon)
npm run build:linux    # Linux:   .deb + .AppImage + .rpm
```

Each script downloads the platform-specific FFmpeg sidecar automatically
via `scripts/fetch-ffmpeg.js` before invoking `tauri build`.

## Build Outputs

After a successful build, installers are placed in:

```
src-tauri/target/release/bundle/
├── msi/      pdf2vid_<version>_x64_en-US.msi
├── nsis/     pdf2vid_<version>_x64-setup.exe
├── dmg/      pdf2vid_<version>_<arch>.dmg
├── macos/    pdf2vid.app
├── deb/      pdf2vid_<version>_amd64.deb
├── appimage/ pdf2vid_<version>_amd64.AppImage
└── rpm/      pdf2vid-<version>-1.<arch>.rpm
```

## FFmpeg Sidecar

`scripts/fetch-ffmpeg.js` downloads platform-specific FFmpeg static
builds and places them in `src-tauri/binaries/` with Tauri's expected
target-triple naming.

| Platform | Source |
|---|---|
| Windows x86_64 | github.com/BtbN/FFmpeg-Builds (GPL) |
| macOS x86_64   | evermeet.cx |
| macOS arm64    | osxexperts.net |
| Linux x86_64   | johnvansickle.com static builds |

Use `npm run fetch-ffmpeg:all` to download for all platforms at once.

## Cross-Platform Builds

Tauri cannot cross-compile desktop apps because of native dependencies
(WebView2 on Windows, WebKit on macOS/Linux). Each platform must be
built on that platform or via CI.

The `.github/workflows/release.yml` workflow builds all three platforms
in parallel when you push a `v*` tag.

## CI Release Flow

1. Bump version in `package.json` and `src-tauri/Cargo.toml`.
2. Update `CHANGELOG.md` with the release notes.
3. Commit and push a `vX.Y.Z` tag.
4. `.github/workflows/release.yml` triggers:
   - Windows installer (`.msi` + `.exe`)
   - macOS bundles (`.dmg` + `.app` for x86_64 and arm64)
   - Linux packages (`.deb` + `.AppImage` + `.rpm`)
   - GitHub release draft with all artifacts.

## Debug Build

For faster iteration during development:

```bash
npm run tauri build -- --debug
```

Produces a debug-mode binary in `src-tauri/target/debug/`. Useful for
testing without waiting for the full release build.

## Web-Only Development

To iterate on the UI without the Rust shell:

```bash
npm run dev
```

Opens the Vite dev server at `http://localhost:1420`. Tauri commands
will fail in this mode — use `npm run tauri dev` for full integration.

## Troubleshooting

**FFmpeg not found at runtime** — Install FFmpeg on the system PATH or
re-run the build with the sidecar fetch step. See
`scripts/fetch-ffmpeg.js`.

**edge-tts not detected** — Install Python 3.8+ and run
`pip install edge-tts`. The status bar will show "edge-tts ready" when
the package is importable.

**WebView2 missing on Windows** — Preinstalled on Windows 10+; for older
systems, install the Evergreen runtime from Microsoft.

**macOS code signing** — Unsigned builds trigger Gatekeeper warnings.
Run `xcrun notarytool submit` with an Apple Developer ID for distribution.