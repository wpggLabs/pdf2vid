use crate::cloud;
use crate::edgetts;
use crate::ffmpeg::{check_ffmpeg, check_ffprobe};
use crate::models;
use crate::providers::provider_list;
use crate::render;
use crate::state::AppState;
use crate::types::{
    ExportRequest, ModelInfo, Project, ProviderList, SystemStatus,
};
use base64::Engine as _;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

fn project_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("current-project.json"))
}

#[tauri::command]
pub fn system_status() -> SystemStatus {
    SystemStatus {
        ffmpeg: check_ffmpeg(),
        ffprobe: check_ffprobe(),
        platform: std::env::consts::OS.to_string(),
        ffmpeg_sidecar_ready: crate::ffmpeg::ffmpeg_path()
            .map(|p| p.parent().map(|dir| dir.join("ffmpeg").exists() || dir.join("ffmpeg.exe").exists()).unwrap_or(false))
            .unwrap_or(false),
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
    serde_json::from_slice(&data).map(Some).map_err(|e| e.to_string())
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
pub fn dependency_status() -> DependencyStatus {
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
                message: "Python 3.8+ with the edge-tts package is the default voice engine.".into(),
                command: "winget install Python.Python.3.12 && pip install edge-tts".into(),
            },
            "macos" => InstallHint {
                tool: "python".into(),
                message: "Python 3.8+ with the edge-tts package is the default voice engine.".into(),
                command: "brew install python@3.12 && pip3 install edge-tts".into(),
            },
            _ => InstallHint {
                tool: "python".into(),
                message: "Python 3.8+ with the edge-tts package is the default voice engine.".into(),
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
        edge_tts_version,
        platform: std::env::consts::OS.to_string(),
        install_hints: hints,
    }
}

fn probe_edge_tts_version(python: &std::path::Path) -> Option<String> {
    let out = std::process::Command::new(python)
        .args(["-c", "import edge_tts; print(edge_tts.__version__ if hasattr(edge_tts, '__version__') else 'installed')"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
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

/// Read a PDF file from disk and return its bytes. Frontend uses this for
/// large PDFs that would be slow or memory-heavy to load via the browser
/// file input.
#[tauri::command]
pub fn read_pdf_file(path: String) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Could not read {path}: {e}"))?;
    Ok(bytes)
}

#[tauri::command]
pub fn list_models(app: AppHandle) -> Vec<ModelInfo> {
    models::list_models(&app)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    model_id: String,
) -> Result<String, String> {
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
    let req = cloud::TranslationRequest {
        text,
        target_language: target_language.clone(),
    };
    if provider == "marian" || provider.is_empty() {
        let resp = cloud::marian_translate("", req).await?;
        return Ok(resp.translated_text);
    }
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
) -> Result<String, String> {
    let audio = match provider.as_str() {
        "edge" => {
            let resp = edgetts::synthesize(edgetts::TtsRequest {
                text,
                voice,
                rate: None,
                pitch: None,
            })
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
            let resp = cloud::openai_tts(&key, cloud::CloudSynthesisRequest {
                text,
                voice,
            })
            .await?;
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
            let resp = cloud::elevenlabs_tts(&key, &voice_id, cloud::CloudSynthesisRequest {
                text,
                voice,
            })
            .await?;
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                resp.audio_base64.as_bytes(),
            )
            .map_err(|e| e.to_string())?
        }
        _ => return Err(format!("Voice provider '{provider}' not implemented")),
    };
    Ok(format!(
        "data:audio/mpeg;base64,{}",
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