# Building pdf2vid installers

This document covers building installable artifacts for Windows, macOS, and Linux.

## Quick start (current platform)

```bash
npm install
npm run build:win     # Windows: produces .msi + .exe
npm run build:mac     # macOS:   produces .dmg + .app
npm run build:linux   # Linux:   produces .deb + .AppImage + .rpm
```

The `build:*` scripts run `node scripts/fetch-ffmpeg.js <target>` first to
download the FFmpeg static binary that the app expects. The binary is placed
in `src-tauri/binaries/` and Tauri's bundler copies it alongside the
executable at build time.

## Manual FFmpeg fetch

If you only want the FFmpeg sidecar (not the full installer):

```bash
npm run fetch-ffmpeg              # current host only
npm run fetch-ffmpeg:all          # win + mac (both arches) + linux
```

The script downloads from these trusted sources:

| Platform | Source |
|---|---|
| Windows x86_64 | BtbN FFmpeg-Builds (gpl) |
| macOS x86_64 | evermeet.cx |
| macOS arm64 | osxexperts.net |
| Linux x86_64 | johnvansickle.com static builds |

## Manual build

```bash
npm install
npm run tauri build
```

Produces:
- **Windows**: `src-tauri/target/release/bundle/{msi,nsis}/*.msi/*.exe`
- **macOS**:   `src-tauri/target/release/bundle/{dmg,macos}/*.dmg/*.app`
- **Linux**:   `src-tauri/target/release/bundle/{deb,rpm,appimage}/*`

## Cross-platform builds

Tauri cannot cross-compile desktop apps because of native dependencies
(WebView2 on Windows, WebKit on macOS/Linux). Each platform must be built on
that platform or via CI.

The `.github/workflows/release.yml` workflow builds all three platforms in
parallel when you push a `v*` tag.

## Toolchain requirements

- **Node.js** 20+
- **Rust** stable (`rustup default stable`)
- **Tauri 2 prerequisites** per platform — see https://v2.tauri.app/start/prerequisites/
- **Linux extra**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
- **Windows**: WebView2 runtime (preinstalled on Win10+)
- **macOS**: Xcode Command Line Tools

## End-user runtime requirement

The desktop app uses system FFmpeg if found, and falls back to the bundled
sidecar. If neither is present, the status bar shows
"FFmpeg not found" and export is blocked. End users can install FFmpeg:

| Platform | Command |
|---|---|
| Windows | `winget install Gyan.FFmpeg` or download from gyan.dev |
| macOS   | `brew install ffmpeg` |
| Linux   | `sudo apt install ffmpeg` (Debian/Ubuntu) or `sudo dnf install ffmpeg` (Fedora) |

## CI release flow

1. Bump version in `package.json` and `src-tauri/Cargo.toml`.
2. Commit and push a `v0.1.0` tag.
3. `.github/workflows/release.yml` triggers:
   - Builds Windows installer (`.msi` + `.exe`)
   - Builds macOS bundles (`.dmg` + `.app` for x86_64 and arm64)
   - Builds Linux packages (`.deb` + `.AppImage` + `.rpm`)
   - Creates a GitHub release draft with all artifacts.

## Local debug build (faster, no installer)

```bash
npm run tauri build -- --debug
```

Produces a debug-mode binary in `src-tauri/target/debug/`. Useful for
testing without waiting for the full release build.