use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::Command};
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemStatus {
    ffmpeg: bool,
    ffprobe: bool,
    platform: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Scene {
    id: String,
    page: u32,
    title: String,
    script: String,
    duration: u32,
    selected: bool,
    thumbnail: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    name: String,
    source_name: String,
    scenes: Vec<Scene>,
    language: String,
    translation_provider: String,
    voice_provider: String,
    voice: String,
    output_you_tube: bool,
    output_tik_tok: bool,
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn project_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join("current-project.json"))
}

#[tauri::command]
fn system_status() -> SystemStatus {
    SystemStatus {
        ffmpeg: command_exists("ffmpeg"),
        ffprobe: command_exists("ffprobe"),
        platform: std::env::consts::OS.to_string(),
    }
}

#[tauri::command]
fn save_project(app: tauri::AppHandle, project: Project) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(&project).map_err(|error| error.to_string())?;
    fs::write(project_path(&app)?, data).map_err(|error| error.to_string())
}

#[tauri::command]
fn load_project(app: tauri::AppHandle) -> Result<Option<Project>, String> {
    let path = project_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&data).map(Some).map_err(|error| error.to_string())
}

#[tauri::command]
fn store_api_key(provider: String, secret: String) -> Result<(), String> {
    if provider.trim().is_empty() || secret.trim().is_empty() {
        return Err("Provider and API key are required".into());
    }
    keyring::Entry::new("com.wpgglabs.pdf2vid", &provider)
        .map_err(|error| error.to_string())?
        .set_password(&secret)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn validate_export(project: Project) -> Result<String, String> {
    if !project.output_you_tube && !project.output_tik_tok {
        return Err("Select at least one output format".into());
    }
    if !project.scenes.iter().any(|scene| scene.selected) {
        return Err("Select at least one scene".into());
    }
    if project.scenes.iter().filter(|scene| scene.selected).any(|scene| scene.script.trim().is_empty()) {
        return Err("Every selected scene needs narration text".into());
    }
    if !command_exists("ffmpeg") || !command_exists("ffprobe") {
        return Err("FFmpeg and FFprobe must be installed before rendering".into());
    }
    Ok("Project validated and ready to render".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![system_status, save_project, load_project, store_api_key, validate_export])
        .run(tauri::generate_context!())
        .expect("failed to run pdf2vid");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        Project {
            name: "Test".into(), source_name: "test.pdf".into(),
            scenes: vec![Scene { id: "1".into(), page: 1, title: "Scene".into(), script: "Narration".into(), duration: 4, selected: true, thumbnail: "".into() }],
            language: "English (US)".into(), translation_provider: "argos".into(), voice_provider: "piper".into(), voice: "Amy".into(),
            output_you_tube: true, output_tik_tok: true,
        }
    }

    #[test]
    fn rejects_no_output() {
        let mut value = project();
        value.output_you_tube = false;
        value.output_tik_tok = false;
        assert_eq!(validate_export(value).unwrap_err(), "Select at least one output format");
    }

    #[test]
    fn rejects_empty_script() {
        let mut value = project();
        value.scenes[0].script.clear();
        assert_eq!(validate_export(value).unwrap_err(), "Every selected scene needs narration text");
    }
}
