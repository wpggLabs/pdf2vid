pub mod types;
pub mod state;
pub mod providers;
pub mod models;
pub mod edgetts;
pub mod cloud;
pub mod ffmpeg;
pub mod render;
pub mod commands;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::system_status,
            commands::save_project,
            commands::load_project,
            commands::store_api_key,
            commands::list_providers,
            commands::list_models,
            commands::is_model_installed,
            commands::download_model,
            commands::delete_model,
            commands::translate_text,
            commands::preview_voice,
            commands::validate_export,
            commands::start_export,
            commands::cancel_export,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run pdf2vid");
}

#[cfg(test)]
mod tests {
    use crate::commands::validate_export;
    use crate::types::{Project, Scene};

    fn project() -> Project {
        Project {
            name: "Test".into(),
            source_name: "test.pdf".into(),
            scenes: vec![Scene {
                id: "1".into(),
                page: 1,
                title: "Scene".into(),
                script: "Narration".into(),
                translated_script: None,
                duration: 4,
                selected: true,
                thumbnail: "".into(),
            }],
            language: "English (US)".into(),
            translation_provider: "marian".into(),
            voice_provider: "edge".into(),
            voice: "en-US-JennyNeural".into(),
            output_you_tube: true,
            output_tik_tok: true,
        }
    }

    #[test]
    fn rejects_no_output() {
        let mut p = project();
        p.output_you_tube = false;
        p.output_tik_tok = false;
        assert_eq!(
            validate_export(p).unwrap_err(),
            "Select at least one output format"
        );
    }

    #[test]
    fn rejects_empty_script() {
        let mut p = project();
        p.scenes[0].script.clear();
        assert_eq!(
            validate_export(p).unwrap_err(),
            "Every selected scene needs narration text"
        );
    }
}