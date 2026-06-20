use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub ffmpeg: bool,
    pub ffprobe: bool,
    pub platform: String,
    pub ffmpeg_sidecar_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: String,
    pub page: u32,
    pub title: String,
    pub script: String,
    pub translated_script: Option<String>,
    pub duration: u32,
    pub selected: bool,
    pub thumbnail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub source_name: String,
    pub scenes: Vec<Scene>,
    pub language: String,
    pub translation_provider: String,
    pub voice_provider: String,
    pub voice: String,
    pub output_you_tube: bool,
    pub output_tik_tok: bool,
    #[serde(default)]
    pub skipped_pages: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Local,
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOption {
    pub id: String,
    pub label: String,
    pub kind: ProviderKind,
    pub detail: String,
    #[serde(default)]
    pub implemented: bool,
    #[serde(default)]
    pub online: bool,
    pub key_label: Option<String>,
    pub category: ProviderCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderCategory {
    Translation,
    Voice,
    Visual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderList {
    pub translation: Vec<ProviderOption>,
    pub voice: Vec<ProviderOption>,
    pub visual: Vec<ProviderOption>,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub project: Project,
    pub output_dir: String,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub job_id: String,
    pub stage: String,
    pub message: String,
    pub percent: u8,
    pub current: Option<u32>,
    pub total: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportComplete {
    pub job_id: String,
    pub youtube_path: Option<String>,
    pub tiktok_path: Option<String>,
    /// Scenes whose translation provider reported an error. The source script
    /// is used as a fallback so the export still completes; the UI should
    /// surface this list so the user knows the translation didn't run.
    #[serde(default)]
    pub translation_warnings: Vec<TranslationWarning>,
    /// Number of pages that were skipped during import because they had no
    /// selectable text. Persisted on Project so the count survives reload.
    #[serde(default)]
    pub skipped_pages: Vec<u32>,
    /// Total scenes that ended up untranslated (computed from translation_warnings).
    #[serde(default)]
    pub untranslated_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationWarning {
    pub scene_id: String,
    pub page: u32,
    pub provider: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportError {
    pub job_id: String,
    pub stage: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub family: String,
    pub label: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub license: String,
    pub requires_accept: bool,
    pub installed: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: u64,
    pub percent: u8,
}