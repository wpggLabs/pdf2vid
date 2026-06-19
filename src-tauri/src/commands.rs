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
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
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
pub fn is_model_installed(app: AppHandle, model_id: String) -> bool {
    crate::models::is_model_installed(&app, &model_id)
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
    let cancel = Arc::new(AtomicBool::new(false));
    let model_id_clone = model_id.clone();
    let app_clone = app.clone();
    let cancel_clone = cancel.clone();
    let result = tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Handle::current();
        runtime.block_on(async move {
            models::download_model(&app_clone, &model_id_clone, cancel_clone).await
        })
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?;
    let _ = app.emit(
        "model:complete",
        serde_json::json!({"modelId": model_id, "success": result.is_ok()}),
    );
    result
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
    Ok(state.cancel_job().await)
}