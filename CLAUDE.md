# pdf2vid

Desktop app that converts PDFs to video. Built with Tauri 2 + React 19 + Rust.

## Stack
- Tauri 2 (Rust backend + webview)
- React 19 + TypeScript (frontend)
- Vite (bundler)

## Key dirs
- `src/` — React frontend
- `src-tauri/` — Rust backend (Tauri commands, file I/O)
- `src-tauri/src/` — Rust source

## Dev
```bash
npm run tauri dev     # full dev (Rust + frontend)
npm run dev           # frontend only
cargo build           # Rust only (from src-tauri/)
```

## Rules
- File system access goes through Tauri commands (IPC), never direct JS fs
- Rust code handles all PDF parsing and FFmpeg invocation
- Use `tauri::command` with proper argument types — no raw shell exec
