use crate::chatterbox;
use crate::cloud;
use crate::edgetts;
use crate::ffmpeg::{check_ffmpeg, check_ffprobe};
use crate::kokoro;
use crate::models;
use crate::providers::provider_list;
use crate::render;
use crate::state::AppState;
use crate::types::{ExportRequest, ModelInfo, Project, ProviderList, SystemStatus};
use base64::Engine as _;
use std::path::PathBuf;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager};

fn project_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("current-project.json"))
}

#[tauri::command]
pub fn system_status() -> SystemStatus {
    // Font discovery reads from the bundled render cache. We use a
    // deterministic temp dir here because system_status runs at app
    // startup before any cache layout exists; the resolver is cheap
    // enough that this is fine, and the discovery stays portable
    // because it only inspects well-known system font directories.
    let font_probe_dir = std::env::temp_dir().join("pdf2vid-font-probe");
    let _ = std::fs::create_dir_all(&font_probe_dir);
    let font = crate::font::resolve_font(&font_probe_dir);
    SystemStatus {
        ffmpeg: check_ffmpeg(),
        ffprobe: check_ffprobe(),
        platform: std::env::consts::OS.to_string(),
        ffmpeg_sidecar_ready: crate::ffmpeg::ffmpeg_path()
            .map(|p| {
                p.parent()
                    .map(|dir| dir.join("ffmpeg").exists() || dir.join("ffmpeg.exe").exists())
                    .unwrap_or(false)
            })
            .unwrap_or(false),
        font_available: font.found,
        font_path: font.render_path,
    }
}

#[tauri::command]
pub fn save_project(app: AppHandle, project: Project) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(&project).map_err(|e| e.to_string())?;
    std::fs::write(project_path(&app)?, data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_project(app: AppHandle) -> Result<Option<Project>, String> {
    let path = project_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn store_api_key(provider: String, secret: String) -> Result<(), String> {
    if provider.trim().is_empty() || secret.trim().is_empty() {
        return Err("Provider and API key are required".into());
    }
    keyring::Entry::new("com.wpgglabs.pdf2vid", &provider)
        .map_err(|e| e.to_string())?
        .set_password(&secret)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_providers() -> ProviderList {
    provider_list()
}

#[tauri::command]
pub fn check_tts_engine() -> TtsEngineStatus {
    let python = crate::edgetts::detect_python_with_edge_tts();
    TtsEngineStatus {
        python_available: python.is_some(),
        python_path: python.map(|p| p.to_string_lossy().to_string()),
        edge_tts_version: None,
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsEngineStatus {
    pub python_available: bool,
    pub python_path: Option<String>,
    pub edge_tts_version: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub ffmpeg: bool,
    pub ffprobe: bool,
    pub ffmpeg_path: Option<String>,
    pub python: bool,
    pub python_path: Option<String>,
    pub edge_tts: bool,
    pub edge_tts_version: Option<String>,
    pub ocr_ready: bool,
    pub platform: String,
    pub install_hints: Vec<InstallHint>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallHint {
    pub tool: String,
    pub message: String,
    pub command: String,
}

#[tauri::command]
pub fn dependency_status(app: AppHandle) -> DependencyStatus {
    let ffmpeg_path = crate::ffmpeg::ffmpeg_path();
    let ffprobe_path = crate::ffmpeg::ffprobe_path();
    let python = crate::edgetts::detect_python_with_edge_tts();

    let ffmpeg = ffmpeg_path.is_some();
    let ffprobe = ffprobe_path.is_some();
    let (python_ok, edge_tts_version) = match python.as_ref() {
        Some(p) => {
            let version = probe_edge_tts_version(p);
            (true, version)
        }
        None => (false, None),
    };

    let mut hints = Vec::new();
    if !ffmpeg {
        hints.push(match std::env::consts::OS {
            "windows" => InstallHint {
                tool: "ffmpeg".into(),
                message: "FFmpeg is required to render videos. Install via winget or download from gyan.dev.".into(),
                command: "winget install Gyan.FFmpeg".into(),
            },
            "macos" => InstallHint {
                tool: "ffmpeg".into(),
                message: "FFmpeg is required to render videos. Install via Homebrew.".into(),
                command: "brew install ffmpeg".into(),
            },
            _ => InstallHint {
                tool: "ffmpeg".into(),
                message: "FFmpeg is required to render videos. Install via your package manager.".into(),
                command: "sudo apt install ffmpeg".into(),
            },
        });
    }
    if !ffprobe {
        hints.push(InstallHint {
            tool: "ffprobe".into(),
            message: "ffprobe (bundled with ffmpeg) is required to probe output videos.".into(),
            command: "Install ffmpeg (ffprobe is included).".into(),
        });
    }
    if !python_ok {
        hints.push(match std::env::consts::OS {
            "windows" => InstallHint {
                tool: "python".into(),
                message: "Python 3.8+ with the edge-tts package is the default voice engine."
                    .into(),
                command: "winget install Python.Python.3.12 && pip install edge-tts".into(),
            },
            "macos" => InstallHint {
                tool: "python".into(),
                message: "Python 3.8+ with the edge-tts package is the default voice engine."
                    .into(),
                command: "brew install python@3.12 && pip3 install edge-tts".into(),
            },
            _ => InstallHint {
                tool: "python".into(),
                message: "Python 3.8+ with the edge-tts package is the default voice engine."
                    .into(),
                command: "sudo apt install python3 python3-pip && pip3 install edge-tts".into(),
            },
        });
    }

    DependencyStatus {
        ffmpeg,
        ffprobe,
        ffmpeg_path: ffmpeg_path.map(|p| p.to_string_lossy().to_string()),
        python: python_ok,
        python_path: python.as_ref().map(|p| p.to_string_lossy().to_string()),
        edge_tts: python_ok,
        ocr_ready: ocr_venv_dir(&app)
            .map(|venv| crate::ocr::ocr_available(&venv))
            .unwrap_or(false),
        edge_tts_version,
        platform: std::env::consts::OS.to_string(),
        install_hints: hints,
    }
}

/// An optional local model provider the user can install to unlock a
/// feature. Surfaced in the UI with a copyable install command.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDep {
    pub id: String,
    pub label: String,
    pub purpose: String,
    pub installed: bool,
    pub command: String,
    pub docs: String,
}

/// Detect whether the optional local providers (Argos translation, Kokoro
/// and Chatterbox voices) are installed, and return the exact pip command
/// to get each one. Detection uses `importlib.util.find_spec`, which
/// checks importability *without* importing the (heavy) package, so this
/// stays fast even when torch-based models are installed.
#[tauri::command]
pub fn local_deps() -> Vec<LocalDep> {
    let python = find_any_python();
    let check = |module: &str| {
        python
            .as_ref()
            .map(|p| module_available(p, module))
            .unwrap_or(false)
    };
    vec![
        LocalDep {
            id: "argos".into(),
            label: "Argos Translate".into(),
            purpose: "Offline translation for non-English output".into(),
            installed: check("argostranslate"),
            command: "pip install argostranslate".into(),
            docs: "https://github.com/argosopentech/argos-translate".into(),
        },
        LocalDep {
            id: "kokoro".into(),
            label: "Kokoro voice".into(),
            purpose: "Fast local voice · 8 languages · runs on CPU".into(),
            installed: check("kokoro"),
            command: "pip install kokoro soundfile".into(),
            docs: "https://github.com/hexgrad/kokoro".into(),
        },
        LocalDep {
            id: "chatterbox".into(),
            label: "Chatterbox voice".into(),
            purpose: "Premium multilingual voice · 23 languages · GPU recommended".into(),
            installed: check("chatterbox"),
            command: "pip install chatterbox-tts torchaudio".into(),
            docs: "https://github.com/resemble-ai/chatterbox".into(),
        },
    ]
}

/// Find any Python 3 interpreter on PATH (independent of installed packages).
fn find_any_python() -> Option<std::path::PathBuf> {
    let candidates = if cfg!(windows) {
        ["python", "python3", "py"]
    } else {
        ["python3", "python", "py"]
    };
    for name in candidates {
        let mut cmd = std::process::Command::new(name);
        cmd.args(["-c", "import sys; sys.exit(0)"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        crate::subprocess::hide_window(&mut cmd);
        if cmd.status().map(|s| s.success()).unwrap_or(false) {
            return Some(std::path::PathBuf::from(name));
        }
    }
    None
}

/// Whether `module` is importable by `python`, checked cheaply via
/// `find_spec` (does not import the module).
fn module_available(python: &std::path::Path, module: &str) -> bool {
    let code = format!(
        "import importlib.util,sys; sys.exit(0 if importlib.util.find_spec('{module}') else 1)"
    );
    let mut cmd = std::process::Command::new(python);
    cmd.args(["-c", &code])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    crate::subprocess::hide_window(&mut cmd);
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn probe_edge_tts_version(python: &std::path::Path) -> Option<String> {
    let mut cmd = std::process::Command::new(python);
    cmd.args(["-c", "import edge_tts; print(edge_tts.__version__ if hasattr(edge_tts, '__version__') else 'installed')"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    crate::subprocess::hide_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

#[tauri::command]
pub fn is_model_installed(app: AppHandle, model_id: String) -> bool {
    crate::models::is_model_installed(&app, &model_id)
}

/// OCR a page image (base64 PNG data URL) and return the recognized text.
/// Used as a fallback when a PDF page has no selectable text (scanned PDFs)
/// so the page can still be narrated instead of being skipped.
///
/// Async + `spawn_blocking`: sync Tauri commands run on the main thread,
/// and OCR shells out to Python for seconds at a time — doing that on the
/// main thread would freeze the whole UI.
#[tauri::command]
pub async fn ocr_image(app: AppHandle, data_url: String) -> Result<String, String> {
    let venv = ocr_venv_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || crate::ocr::ocr_png_data_url(&venv, &data_url))
        .await
        .map_err(|e| format!("OCR task failed: {e}"))?
}

/// Install the OCR engine in the background if it isn't already present.
/// Returns whether OCR is ready after the attempt. Called at startup so
/// scanned-PDF reading works without the user installing anything.
///
/// Async + `spawn_blocking` for the same reason as [`ocr_image`]: the
/// first-time pip install downloads hundreds of MB and must never run on
/// the main thread.
#[tauri::command]
pub async fn ensure_ocr(app: AppHandle) -> Result<bool, String> {
    let venv = ocr_venv_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::ocr::ensure_ocr_installed(&venv);
        crate::ocr::ocr_available(&venv)
    })
    .await
    .map_err(|e| format!("OCR setup task failed: {e}"))
}

/// Dedicated venv for the OCR engine, under the app's data directory —
/// never the user's global Python environment.
fn ocr_venv_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
    Ok(base.join("ocr-venv"))
}

/// Read a PDF file from disk and return its bytes. Frontend uses this for
/// large PDFs that would be slow or memory-heavy to load via the browser
/// file input.
///
/// Defense-in-depth: this is a Tauri command reachable from the webview, so
/// we restrict it to `.pdf` files and cap the size to avoid an arbitrary-file
/// read or memory-exhaustion DoS.
#[tauri::command]
pub fn read_pdf_file(path: String) -> Result<Response, String> {
    // Canonicalize first: resolves `..` and symlinks so the extension and
    // size checks below operate on the real target, and so a path like
    // `evil.pdf` that is actually a symlink to a non-PDF cannot slip
    // through. The command is intentionally allowed to read any `.pdf` the
    // user selects (including outside the app dir) — that is its purpose.
    let canonical = std::fs::canonicalize(&path)
        .map_err(|e| format!("Could not access {path}: {e}"))?;
    match canonical.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("pdf") => {}
        _ => return Err("Only .pdf files can be read".into()),
    }

    let meta = std::fs::metadata(&canonical)
        .map_err(|e| format!("Could not access {}: {e}", canonical.display()))?;
    if !meta.is_file() {
        return Err("Path is not a regular file".into());
    }
    const MAX_BYTES: u64 = 200 * 1024 * 1024;
    if meta.len() > MAX_BYTES {
        return Err(format!(
            "PDF is too large ({} MB, limit is 200 MB)",
            meta.len() / (1024 * 1024)
        ));
    }

    let bytes =
        std::fs::read(&canonical).map_err(|e| format!("Could not read {}: {e}", canonical.display()))?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub fn list_models(app: AppHandle) -> Vec<ModelInfo> {
    models::list_models(&app)
}

#[tauri::command]
pub async fn download_model(app: AppHandle, model_id: String) -> Result<String, String> {
    // Per-download cancel flag is registered in the global AppState. This
    // is the same pattern as the export job so the user can cancel a long
    // model download from the UI. If the user starts a second download the
    // first flag is set so the older one cleans up.
    let state = app.state::<AppState>();
    let cancel = state.start_model_download(model_id.clone()).await;

    let result = models::download_model(&app, &model_id, cancel).await;

    state.finish_model_download(&model_id).await;

    let _ = app.emit(
        "model:complete",
        serde_json::json!({"modelId": model_id, "success": result.is_ok()}),
    );
    result
}

#[tauri::command]
pub async fn cancel_model_download(app: AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<AppState>();
    Ok(state.cancel_model_download().await)
}

#[tauri::command]
pub fn delete_model(app: AppHandle, model_id: String) -> Result<(), String> {
    models::delete_model(&app, &model_id)
}

#[tauri::command]
pub async fn translate_text(
    provider: String,
    target_language: String,
    text: String,
) -> Result<String, String> {
    // Local/offline default: Argos. (Legacy `marian` and empty provider
    // route here too, so older saved projects keep working.)
    if provider == "argos" || provider == "marian" || provider.is_empty() {
        let to = crate::providers::argos_lang_code(&target_language);
        return crate::argos::translate("en", to, &text, &|_| {}).await;
    }
    let req = cloud::TranslationRequest {
        text,
        target_language: target_language.clone(),
    };
    let key = keyring::Entry::new("com.wpgglabs.pdf2vid", &provider)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| format!("No API key for {provider}"))?;
    let resp = match provider.as_str() {
        "openai" => cloud::openai_translate(&key, req).await?,
        "google" => cloud::google_translate(&key, req).await?,
        _ => return Err(format!("{provider} translator not implemented")),
    };
    Ok(resp.translated_text)
}

#[tauri::command]
pub async fn preview_voice(
    provider: String,
    voice: String,
    text: String,
    speed: Option<u32>,
) -> Result<String, String> {
    let audio = match provider.as_str() {
        "edge" => {
            let resp = match edgetts::synthesize(edgetts::TtsRequest {
                text,
                voice,
                rate: render::speed_to_rate(speed.unwrap_or(100)),
                pitch: None,
            })
            .await
            {
                Ok(r) => r,
                Err(e) => return Err(format!("edge-tts synthesis failed: {e}")),
            };
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                resp.audio_base64.as_bytes(),
            )
            .map_err(|e| e.to_string())?
        }
        "kokoro" => {
            // A kokoro voice id's first letter is its language code
            // (e.g. `af_heart` -> `a`).
            let lang = voice.chars().next().map(String::from).unwrap_or_default();
            let resp = kokoro::synthesize(
                kokoro::KokoroRequest {
                    text,
                    voice,
                    lang_code: lang,
                    speed: speed.unwrap_or(100) as f32 / 100.0,
                },
                &|_| {},
            )
            .await?;
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                resp.audio_base64.as_bytes(),
            )
            .map_err(|e| e.to_string())?
        }
        "chatterbox" => {
            // Frontend sets the voice value to the language id.
            let resp = chatterbox::synthesize(
                chatterbox::ChatterboxRequest {
                    text,
                    language_id: voice,
                },
                &|_| {},
            )
            .await?;
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                resp.audio_base64.as_bytes(),
            )
            .map_err(|e| e.to_string())?
        }
        "piper" => {
            return Err("Piper preview not yet implemented".into());
        }
        "openai" => {
            let key = keyring::Entry::new("com.wpgglabs.pdf2vid", "openai")
                .map_err(|e| e.to_string())?
                .get_password()
                .map_err(|_| "No OpenAI API key".to_string())?;
            let resp =
                cloud::openai_tts(&key, cloud::CloudSynthesisRequest { text, voice }).await?;
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                resp.audio_base64.as_bytes(),
            )
            .map_err(|e| e.to_string())?
        }
        "elevenlabs" => {
            let key = keyring::Entry::new("com.wpgglabs.pdf2vid", "elevenlabs")
                .map_err(|e| e.to_string())?
                .get_password()
                .map_err(|_| "No ElevenLabs API key".to_string())?;
            let voice_id = voice.clone();
            let resp = cloud::elevenlabs_tts(
                &key,
                &voice_id,
                cloud::CloudSynthesisRequest { text, voice },
            )
            .await?;
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                resp.audio_base64.as_bytes(),
            )
            .map_err(|e| e.to_string())?
        }
        _ => return Err(format!("Voice provider '{provider}' not implemented")),
    };
    // Kokoro and Chatterbox emit WAV; the others emit MP3. Label the data
    // URL correctly so the browser's <audio> element decodes it reliably.
    let mime = match provider.as_str() {
        "kokoro" | "chatterbox" => "audio/wav",
        _ => "audio/mpeg",
    };
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&audio)
    ))
}

#[tauri::command]
pub fn validate_export(project: Project) -> Result<String, String> {
    if !project.output_you_tube && !project.output_tik_tok {
        return Err("Select at least one output format".into());
    }
    if !project.scenes.iter().any(|s| s.selected) {
        return Err("Select at least one scene".into());
    }
    if project
        .scenes
        .iter()
        .filter(|s| s.selected)
        .any(|s| s.script.trim().is_empty())
    {
        return Err("Every selected scene needs narration text".into());
    }
    if !check_ffmpeg() || !check_ffprobe() {
        return Err("FFmpeg and FFprobe must be installed before rendering".into());
    }
    Ok("Project validated and ready to render".into())
}

#[tauri::command]
pub async fn start_export(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    request: ExportRequest,
) -> Result<crate::types::ExportComplete, String> {
    // Only one export may run at a time. Overlapping exports would write to
    // the same cache/output directories and corrupt each other. The caller
    // must cancel the in-flight export first.
    {
        let active = state.active_job.lock().await;
        if active.is_some() {
            return Err("An export is already running; cancel it before starting a new one".into());
        }
    }
    render::run_export(app, state, request).await
}

#[tauri::command]
pub async fn cancel_export(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let job_id = state.cancel_job().await;
    // Also kill any active FFmpeg child. cancel_job sets the flag which
    // the render task polls; start_kill is the immediate signal that
    // terminates the process even if the render task is mid-FFmpeg-wait.
    if let Some(slot) = state.take_ffmpeg_child().await {
        let mut guard = slot.lock().await;
        if let Some(child) = guard.as_mut() {
            let _ = child.start_kill();
        }
    }
    Ok(job_id)
}
