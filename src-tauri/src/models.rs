use crate::providers::provider_list;
use crate::state::app_data_dir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub id: String,
    pub family: String,
    pub label: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub license: String,
    pub requires_accept: bool,
}

// The model registry is intentionally empty. It previously listed 9
// MarianMT language pairs and 1 Piper voice, but none of them were
// real: the file names requested didn't exist in the upstream Hugging
// Face repos (confirmed 404 on every one of them), and even a
// successful download would have gone unused — `translate_text`
// aliases the "marian" provider straight to Argos
// (commands.rs), and `cloud::marian_translate` /
// `cloud::piper_synthesize` unconditionally return "not yet
// implemented". Neither provider is offered in the translation/voice
// dropdowns (providers.rs marks Piper "Coming soon" and Marian isn't
// listed at all), so this only affected users who opened the Local
// Models modal directly.
//
// Re-populate this once local ONNX inference is actually wired up
// (see the integration points noted in cloud.rs), with real,
// currently-hosted file names and pinned sha256 hashes per file.
fn model_registry() -> Vec<ModelSpec> {
    vec![]
}

/// Reject model IDs that could escape the models directory (path
/// separators or `..` segments). Model IDs are registry-generated
/// identifiers (`marian-en-es`, `piper-en_US-amy`), never paths.
fn validate_model_id(model_id: &str) -> Result<(), String> {
    if model_id.is_empty()
        || model_id.contains('/')
        || model_id.contains('\\')
        || model_id.contains("..")
        || model_id.starts_with('.')
    {
        return Err(format!("Invalid model id: {model_id}"));
    }
    Ok(())
}

pub fn model_path(app: &AppHandle, model_id: &str) -> Result<PathBuf, String> {
    validate_model_id(model_id)?;
    let dir = app_data_dir(app)?.join("models").join(model_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn is_model_installed(app: &AppHandle, model_id: &str) -> bool {
    model_path(app, model_id)
        .map(|p| p.join(".installed").exists())
        .unwrap_or(false)
}

pub fn list_models(app: &AppHandle) -> Vec<crate::types::ModelInfo> {
    model_registry()
        .into_iter()
        .map(|spec| {
            let installed = is_model_installed(app, &spec.id);
            let path = if installed {
                model_path(app, &spec.id)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            } else {
                None
            };
            crate::types::ModelInfo {
                id: spec.id,
                family: spec.family,
                label: spec.label,
                url: spec.url,
                size_bytes: spec.size_bytes,
                sha256: spec.sha256,
                license: spec.license,
                requires_accept: spec.requires_accept,
                installed,
                path,
            }
        })
        .collect()
}

pub fn language_pair_for(language: &str) -> &'static str {
    match language {
        "Spanish" => "en-es",
        "French" => "en-fr",
        "German" => "en-de",
        "Portuguese" => "en-pt",
        "Hindi" => "en-hi",
        "Japanese" => "en-jap",
        "Korean" => "en-ko",
        "Chinese (Simplified)" => "en-zh",
        "Arabic" => "en-ar",
        _ => "en-en",
    }
}

pub fn model_id_for_pair(language: &str) -> String {
    let pair = language_pair_for(language);
    if pair == "en-en" {
        return String::new();
    }
    let parts: Vec<&str> = pair.split('-').collect();
    if parts.len() == 2 {
        format!("marian-{}-{}", parts[0], parts[1])
    } else {
        String::new()
    }
}

pub fn find_spec(id: &str) -> Option<ModelSpec> {
    model_registry().into_iter().find(|m| m.id == id)
}

pub async fn download_model(
    app: &AppHandle,
    model_id: &str,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<String, String> {
    let spec = find_spec(model_id).ok_or_else(|| format!("Unknown model: {model_id}"))?;
    let dir = model_path(app, model_id)?;

    let files = if spec.family == "marian" {
        vec![
            "config.json",
            "tokenizer.json",
            "model.safetensors",
            "vocab.json",
            "source.spm",
            "target.spm",
        ]
    } else if spec.family == "piper" {
        vec!["config.json", "model.onnx", "model.onnx.json"]
    } else {
        return Err(format!("Unknown model family: {}", spec.family));
    };
    let files_count = files.len() as u64;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    for file in files {
        if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("Download cancelled".into());
        }
        let url = format!("{}{}", spec.url, file);
        let target = dir.join(file);
        if target.exists() {
            continue;
        }

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch {url}: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Download failed for {file}: HTTP {}",
                resp.status()
            ));
        }

        let total = resp
            .content_length()
            .unwrap_or(spec.size_bytes / files_count);
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();
        let mut file_handle =
            std::fs::File::create(&target).map_err(|e| format!("Cannot write {target:?}: {e}"))?;

        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                drop(file_handle);
                let _ = std::fs::remove_file(&target);
                return Err("Download cancelled".into());
            }
            let bytes = chunk.map_err(|e| format!("Stream error: {e}"))?;
            std::io::Write::write_all(&mut file_handle, &bytes)
                .map_err(|e| format!("Write error: {e}"))?;
            downloaded += bytes.len() as u64;
            let percent = ((downloaded as f64 / total as f64) * 100.0).min(100.0) as u8;
            let _ = app.emit(
                "model:progress",
                crate::types::ModelDownloadProgress {
                    model_id: model_id.into(),
                    downloaded,
                    total,
                    percent,
                },
            );
        }

        // Verify integrity when the registry ships a hash. This is a no-op
        // while `spec.sha256` is empty, and an active guard against corrupt
        // or tampered downloads otherwise.
        //
        // SECURITY: the model registry currently uses mutable Hugging
        // Face `resolve/main` URLs with no pinned hash, so integrity
        // verification is disabled by default. We surface that loudly
        // rather than silently accepting unverified executable weights.
        if !spec.sha256.is_empty() {
            let actual = hash_file(&target).map_err(|e| e.to_string())?;
            if actual != spec.sha256 {
                let _ = std::fs::remove_file(&target);
                return Err(format!(
                    "Checksum mismatch for {file}: expected {}, got {actual}",
                    spec.sha256
                ));
            }
        } else {
            let _ = app.emit(
                "model:progress",
                crate::types::ModelDownloadProgress {
                    model_id: model_id.into(),
                    downloaded: total,
                    total,
                    percent: 100,
                },
            );
            log::warn!(
                "Model '{}' is downloaded WITHOUT checksum verification \
                 (registry hash is empty and the source URL is a mutable \
                 'resolve/main' ref). Pin a hash before shipping in a \
                 release build.",
                spec.id
            );
        }
    }

    // Mark installed
    std::fs::write(dir.join(".installed"), b"ok").map_err(|e| e.to_string())?;

    Ok(dir.to_string_lossy().to_string())
}

pub fn hash_file(path: &PathBuf) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

pub fn delete_model(app: &AppHandle, model_id: &str) -> Result<(), String> {
    let dir = model_path(app, model_id)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn providers_using_models() -> Vec<String> {
    provider_list()
        .translation
        .into_iter()
        .chain(provider_list().voice)
        .filter(|p| p.id == "marian" || p.id == "piper")
        .map(|p| p.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_pair_for_known_languages() {
        assert_eq!(language_pair_for("Spanish"), "en-es");
        assert_eq!(language_pair_for("Chinese (Simplified)"), "en-zh");
        assert_eq!(language_pair_for("English (US)"), "en-en");
    }

    #[test]
    fn model_id_for_pair_format() {
        assert_eq!(model_id_for_pair("Spanish"), "marian-en-es");
        assert_eq!(model_id_for_pair("Japanese"), "marian-en-jap");
        assert_eq!(model_id_for_pair("English (US)"), "");
    }

    #[test]
    fn registry_is_empty_until_local_inference_is_implemented() {
        // See the comment on model_registry(): every previous entry
        // requested files that 404 against upstream, and none of the
        // families they belonged to (marian, piper) have a working
        // local inference path yet. Guards against silently
        // re-adding broken entries.
        assert!(model_registry().is_empty());
    }
}
