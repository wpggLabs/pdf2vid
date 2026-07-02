use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub ffmpeg: bool,
    pub ffprobe: bool,
    pub platform: String,
    pub ffmpeg_sidecar_ready: bool,
    /// Whether a usable drawtext font was discovered on the host. `false`
    /// means exports will fall back to text-less drawtext.
    #[serde(default)]
    pub font_available: bool,
    /// Human-readable path of the font that will be used, when one exists.
    /// `None` when no font was found.
    #[serde(default)]
    pub font_path: Option<String>,
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
    /// Narration speed as a percentage (100 = normal). The UI slider is
    /// bounded to 75–125. Converted to an edge-tts `--rate` string at
    /// synthesis time. Defaults to 100 for projects saved before this field
    /// existed.
    #[serde(default = "default_voice_speed")]
    pub voice_speed: u32,
}

pub fn default_voice_speed() -> u32 {
    100
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
    /// Structured warnings for every category: skipped pages, missing
    /// fonts, render fallbacks, dependency issues, etc. The frontend
    /// renders these instead of relying on stringly-typed status text.
    #[serde(default)]
    pub warnings: Vec<ProjectWarning>,
    /// `true` when render fell back to text-less drawtext because no font
    /// was available. The frontend can call this out specifically.
    #[serde(default)]
    pub render_fallback_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WarningCode {
    /// PDF page had no selectable text and was dropped from the scene list.
    SkippedPage,
    /// Translation provider returned an error; source script used as fallback.
    UntranslatedScene,
    /// drawtext could not find a font; render proceeded with the fallback path.
    MissingFont,
    /// drawtext errored at render time and we fell back to text-less export.
    RenderFallback,
    /// FFmpeg, ffprobe, python, or edge-tts dependency is not installed.
    MissingDependency,
    /// The user picked a provider that exists in the registry but isn't wired up.
    UnsupportedProvider,
    /// Voice synthesis failed across all providers in the fallback chain.
    VoiceSynthesisFailed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}

/// Structured project warning surfaced through `ExportComplete.warnings`.
///
/// We keep `TranslationWarning` around for the per-scene translation
/// case (where scene_id + page are the natural keys), but every other
/// category of issue flows through `ProjectWarning` so the frontend can
/// render consistent, structured UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWarning {
    pub code: WarningCode,
    pub severity: WarningSeverity,
    /// Scene id, page number, or `None` for project-wide warnings.
    pub scene_id: Option<String>,
    pub page: Option<u32>,
    pub message: String,
    /// Optional short technical detail (e.g. an FFmpeg stderr line or
    /// the actual provider error string). Frontend may hide by default.
    pub detail: Option<String>,
    /// Actionable suggestion, e.g. "Install FFmpeg via winget".
    pub suggested_fix: Option<String>,
}

impl ProjectWarning {
    pub fn info(code: WarningCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: WarningSeverity::Info,
            scene_id: None,
            page: None,
            message: message.into(),
            detail: None,
            suggested_fix: None,
        }
    }

    pub fn warning(code: WarningCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: WarningSeverity::Warning,
            scene_id: None,
            page: None,
            message: message.into(),
            detail: None,
            suggested_fix: None,
        }
    }

    pub fn error(code: WarningCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: WarningSeverity::Error,
            scene_id: None,
            page: None,
            message: message.into(),
            detail: None,
            suggested_fix: None,
        }
    }

    pub fn with_scene(mut self, scene_id: impl Into<String>, page: u32) -> Self {
        self.scene_id = Some(scene_id.into());
        self.page = Some(page);
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.suggested_fix = Some(fix.into());
        self
    }
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
