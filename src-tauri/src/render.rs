use crate::cloud;
use crate::edgetts;
use crate::ffmpeg::{ensure_ffmpeg_or_error, ensure_ffprobe_or_error, Aspect};
use crate::models;
use crate::providers::edge_voice_for_language;
use crate::state::{cache_dir, AppState};
use crate::types::{ExportComplete, ExportError, ExportProgress, ExportRequest, Project, TranslationWarning};
use base64::Engine;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub async fn run_export(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportComplete, String> {
    let job_id = request.job_id.clone();
    let cancel_flag = state.start_job(job_id.clone()).await;

    let result = run_export_inner(app.clone(), state.clone(), cancel_flag.clone(), request).await;

    state.finish_job(&job_id).await;
    state.clear_ffmpeg_child().await;

    match result {
        Ok(complete) => {
            let _ = app.emit("export:done", &complete);
            Ok(complete)
        }
        Err(err) => {
            let payload = ExportError {
                job_id: job_id.clone(),
                stage: "render".into(),
                message: err.clone(),
            };
            let _ = app.emit("export:error", &payload);
            Err(err)
        }
    }
}

async fn run_export_inner(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    cancel: Arc<AtomicBool>,
    request: ExportRequest,
) -> Result<ExportComplete, String> {
    let project = request.project;
    let output_dir = PathBuf::from(&request.output_dir);
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    // Validate
    let _ = ensure_ffmpeg_or_error()?;
    let _ = ensure_ffprobe_or_error()?;
    if !project.output_you_tube && !project.output_tik_tok {
        return Err("Select at least one output format".into());
    }
    let selected: Vec<_> = project.scenes.iter().filter(|s| s.selected).collect();
    if selected.is_empty() {
        return Err("Select at least one scene".into());
    }

    emit_progress(&app, &request.job_id, "Planning", "Preparing project", 2, None, None);

    let cache = cache_dir(&app)?;
    let audio_dir = cache.join("audio");
    let visuals_dir = cache.join("visuals");
    std::fs::create_dir_all(&audio_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&visuals_dir).map_err(|e| e.to_string())?;

    let total = selected.len() as u32;
    let mut current = 0u32;

    // Stage 1: translate
    let mut translation_warnings: Vec<TranslationWarning> = Vec::new();
    let translated_scenes = if project.translation_provider == "marian" || is_default_translator(&project) {
        if !is_english(&project.language) {
            emit_progress(
                &app,
                &request.job_id,
                "Translating",
                &format!("Local MarianMT to {}", project.language),
                5,
                Some(0),
                Some(total),
            );
            // Translate each scene
            let mut translated = Vec::new();
            for scene in &project.scenes {
                if cancel.load(Ordering::SeqCst) {
                    return Err("Cancelled".into());
                }
                if !scene.selected {
                    translated.push(scene.clone());
                    continue;
                }
                current += 1;
                let result = cloud::marian_translate(
                    "",
                    cloud::TranslationRequest {
                        text: scene.script.clone(),
                        target_language: project.language.clone(),
                    },
                )
                .await;
                let mut s = scene.clone();
                match result {
                    Ok(r) => s.translated_script = Some(r.translated_text),
                    Err(e) => {
                        // MarianMT local inference is not implemented yet.
                        // Don't pretend the translation worked: record a warning
                        // and fall back to the source script so the export still
                        // completes (the user gets a video, not a hard error).
                        translation_warnings.push(TranslationWarning {
                            scene_id: scene.id.clone(),
                            page: scene.page,
                            provider: "marian".into(),
                            message: format!(
                                "MarianMT inference is not yet implemented; \
                                 using source text for page {}",
                                scene.page
                            ),
                        });
                        log::warn!("MarianMT failed for page {}: {}", scene.page, e);
                        s.translated_script = Some(scene.script.clone());
                    }
                }
                translated.push(s);
                emit_progress(
                    &app,
                    &request.job_id,
                    "Translating",
                    &format!("Scene {} of {}", current, total),
                    5 + ((current as f64 / total as f64) * 20.0) as u8,
                    Some(current),
                    Some(total),
                );
            }
            translated
        } else {
            project.scenes.clone()
        }
    } else {
        // Cloud translation path
        emit_progress(
            &app,
            &request.job_id,
            "Translating",
            &format!("Cloud translator: {}", project.translation_provider),
            5,
            Some(0),
            Some(total),
        );

        let api_key = read_api_key(&project.translation_provider)?;
        let mut translated = Vec::new();
        for scene in &project.scenes {
            if cancel.load(Ordering::SeqCst) {
                return Err("Cancelled".into());
            }
            if !scene.selected {
                translated.push(scene.clone());
                continue;
            }
            current += 1;
            let req = cloud::TranslationRequest {
                text: scene.script.clone(),
                target_language: project.language.clone(),
            };
            let result = match project.translation_provider.as_str() {
                "openai" => cloud::openai_translate(&api_key, req).await,
                "google" => cloud::google_translate(&api_key, req).await,
                _ => Err(format!(
                    "{} translator is not yet implemented",
                    project.translation_provider
                )),
            };
            let mut s = scene.clone();
            s.translated_script = match result {
                Ok(r) => Some(r.translated_text),
                Err(e) => {
                    return Err(format!(
                        "Translation failed for scene {}: {}",
                        scene.page, e
                    ))
                }
            };
            translated.push(s);
            emit_progress(
                &app,
                &request.job_id,
                "Translating",
                &format!("Scene {} of {}", current, total),
                5 + ((current as f64 / total as f64) * 20.0) as u8,
                Some(current),
                Some(total),
            );
        }
        translated
    };

    // Stage 2: synthesize voice
    let mut audio_paths: Vec<(String, PathBuf)> = Vec::new();
    emit_progress(&app, &request.job_id, "Synthesizing", "Generating narration", 30, Some(0), Some(total));
    current = 0;
    for scene in &translated_scenes {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }
        if !scene.selected {
            continue;
        }
        current += 1;
        let script = scene
            .translated_script
            .clone()
            .unwrap_or_else(|| scene.script.clone());
        let voice_name = resolve_voice(&project, &script);
        let audio_path = audio_dir.join(format!("scene-{}.mp3", scene.id));

        let result = synthesize_scene_audio(
            &app,
            &project,
            &script,
            &voice_name,
            &audio_path,
        )
        .await;

        result?;
        audio_paths.push((scene.id.clone(), audio_path));

        emit_progress(
            &app,
            &request.job_id,
            "Synthesizing",
            &format!("Scene {} of {}", current, total),
            30 + ((current as f64 / total as f64) * 30.0) as u8,
            Some(current),
            Some(total),
        );
    }

    // Stage 3: prepare visuals (PDF thumbnails already exist in Scene.thumbnail)
    emit_progress(&app, &request.job_id, "Visuals", "Preparing page visuals", 65, Some(0), Some(total));
    current = 0;
    let mut visual_paths: Vec<(String, PathBuf)> = Vec::new();
    for scene in &translated_scenes {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }
        if !scene.selected {
            continue;
        }
        current += 1;
        let visual_path = visuals_dir.join(format!("scene-{}.jpg", scene.id));
        if let Some(data) = scene.thumbnail.strip_prefix("data:image/jpeg;base64,") {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|e| e.to_string())?;
            std::fs::write(&visual_path, bytes).map_err(|e| e.to_string())?;
        } else {
            return Err(format!("Scene {} has no valid thumbnail", scene.page));
        }
        visual_paths.push((scene.id.clone(), visual_path));
        emit_progress(
            &app,
            &request.job_id,
            "Visuals",
            &format!("Page {} of {}", current, total),
            65 + ((current as f64 / total as f64) * 10.0) as u8,
            Some(current),
            Some(total),
        );
    }

    // Stage 4: compose videos
    let mut youtube_path = None;
    let mut tiktok_path = None;
    let safe_name = sanitize(&project.name);

    if project.output_you_tube {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }
        emit_progress(&app, &request.job_id, "Composing", "Rendering YouTube 1920×1080", 78, None, None);
        let path = output_dir.join(format!("{}-youtube.mp4", safe_name));
        compose_video(
            state.clone(),
            cancel.clone(),
            &translated_scenes,
            &visual_paths,
            &audio_paths,
            Aspect::Youtube,
            &path,
        )
        .await?;
        youtube_path = Some(path.to_string_lossy().to_string());
    }

    if project.output_tik_tok {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }
        emit_progress(&app, &request.job_id, "Composing", "Rendering TikTok 1080×1920", 90, None, None);
        let path = output_dir.join(format!("{}-tiktok.mp4", safe_name));
        compose_video(
            state.clone(),
            cancel.clone(),
            &translated_scenes,
            &visual_paths,
            &audio_paths,
            Aspect::Tiktok,
            &path,
        )
        .await?;
        tiktok_path = Some(path.to_string_lossy().to_string());
    }

    emit_progress(&app, &request.job_id, "Done", "Export complete", 100, None, None);

    Ok(ExportComplete {
        job_id: request.job_id,
        youtube_path,
        tiktok_path,
        translation_warnings,
    })
}

fn emit_progress(
    app: &AppHandle,
    job_id: &str,
    stage: &str,
    message: &str,
    percent: u8,
    current: Option<u32>,
    total: Option<u32>,
) {
    let payload = ExportProgress {
        job_id: job_id.into(),
        stage: stage.into(),
        message: message.into(),
        percent,
        current,
        total,
    };
    let _ = app.emit("export:progress", &payload);
}

/// Build the FFmpeg argument list for one render. Pure function so the
/// smoke test (and any future test) can verify the filter shape without
/// needing a Tauri runtime.
pub fn build_ffmpeg_args(
    inputs: &[String],
    filter: &str,
    output: &PathBuf,
) -> Vec<String> {
    let mut args = vec!["-y".to_string()];
    args.extend_from_slice(inputs);
    args.push("-filter_complex".to_string());
    args.push(filter.to_string());
    args.push("-map".to_string());
    args.push("[vout]".to_string());
    args.push("-map".to_string());
    args.push("[aout]".to_string());
    args.push("-c:v".to_string());
    args.push("libx264".to_string());
    args.push("-preset".to_string());
    args.push("fast".to_string());
    args.push("-crf".to_string());
    args.push("23".to_string());
    args.push("-pix_fmt".to_string());
    args.push("yuv420p".to_string());
    args.push("-c:a".to_string());
    args.push("aac".to_string());
    args.push("-b:a".to_string());
    args.push("192k".to_string());
    args.push("-movflags".to_string());
    args.push("+faststart".to_string());
    args.push(output.to_string_lossy().to_string());
    args
}

/// Run FFmpeg as a cancellable child process. The child handle is
/// registered in AppState so a concurrent `cancel_export` call can
/// terminate it via `start_kill`. The cancel flag is polled between
/// stages as a secondary signal (in case the OS kill races).
async fn compose_video(
    state: tauri::State<'_, AppState>,
    cancel: Arc<AtomicBool>,
    scenes: &[crate::types::Scene],
    visuals: &[(String, PathBuf)],
    audios: &[(String, PathBuf)],
    aspect: Aspect,
    output: &PathBuf,
) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }

    let ffmpeg = ensure_ffmpeg_or_error()?;
    let (w, h) = aspect.dimensions();
    let mut inputs: Vec<String> = Vec::new();
    let mut filter = String::new();
    let mut audio_inputs: Vec<String> = Vec::new();

    for (i, scene) in scenes.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }
        if !scene.selected {
            continue;
        }
        let visual = visuals
            .iter()
            .find(|(id, _)| id == &scene.id)
            .map(|(_, p)| p.clone())
            .ok_or_else(|| format!("Missing visual for scene {}", scene.page))?;
        let audio = audios
            .iter()
            .find(|(id, _)| id == &scene.id)
            .map(|(_, p)| p.clone())
            .ok_or_else(|| format!("Missing audio for scene {}", scene.page))?;

        inputs.push("-loop".into());
        inputs.push("1".into());
        inputs.push("-i".into());
        inputs.push(visual.to_string_lossy().to_string());
        inputs.push("-i".into());
        inputs.push(audio.to_string_lossy().to_string());

        let v_idx = i * 2;
        let a_idx = i * 2 + 1;
        let script = scene
            .translated_script
            .clone()
            .unwrap_or_else(|| scene.script.clone());
        let safe_script = sanitize_ffmpeg_drawtext(&script);
        let seconds = scene.duration.max(1);

        filter.push_str(&format!(
            "[{v_idx}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black,zoompan=z='min(zoom+0.0008,1.15)':d={seconds}*25:s={w}x{h},drawtext=text='{safe_script}':fontcolor=white:fontsize=42:box=1:boxcolor=black@0.55:boxborderw=14:x=(w-text_w)/2:y=h-80[v{i}];",
            v_idx = v_idx,
            w = w,
            h = h,
            seconds = seconds,
            safe_script = safe_script,
            i = i,
        ));
        filter.push_str(&format!("[{a_idx}:a]aresample=44100[a{i}];", a_idx = a_idx, i = i));
        audio_inputs.push(format!("[v{i}][a{i}]", i = i));
    }

    let concat_inputs = audio_inputs.join("");
    filter.push_str(&format!(
        "{concat_inputs}concat=n={n}:v=1:a=1[vout][aout]",
        concat_inputs = concat_inputs,
        n = audio_inputs.len()
    ));

    let args = build_ffmpeg_args(&inputs, &filter, output);

    let mut cmd = tokio::process::Command::new(&ffmpeg);
    cmd.args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    // CREATE_NO_WINDOW on Windows so FFmpeg doesn't flash a cmd window.
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child_slot: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    state
        .replace_ffmpeg_child_inner(child_slot.clone())
        .await;

    let child = cmd.spawn().map_err(|e| format!("Failed to spawn ffmpeg: {e}"))?;

    // Stash the actual PID into the slot so cancel_export can kill it.
    {
        let mut guard = child_slot.lock().await;
        *guard = Some(child);
    }

    // Wait for the child, polling the cancel flag in case `start_kill`
    // races. Use a tight poll so cancellation feels responsive.
    let status = loop {
        {
            let mut guard = child_slot.lock().await;
            if let Some(c) = guard.as_mut() {
                match c.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) => {}
                    Err(e) => return Err(format!("ffmpeg wait failed: {e}")),
                }
            } else {
                // Someone else took the child; treat as cancellation.
                return Err("Cancelled".into());
            }
        }
        if cancel.load(Ordering::SeqCst) {
            // Best-effort kill and return cancellation error.
            let mut guard = child_slot.lock().await;
            if let Some(c) = guard.as_mut() {
                let _ = c.start_kill();
            }
            return Err("Cancelled".into());
        }
        // 100ms poll keeps UI responsive without burning CPU.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };

    // Drain stderr so the error message has the FFmpeg output if it failed.
    let mut stderr_bytes = Vec::new();
    {
        let mut guard = child_slot.lock().await;
        if let Some(c) = guard.as_mut() {
            if let Some(mut stderr) = c.stderr.take() {
                use tokio::io::AsyncReadExt;
                let _ = stderr.read_to_end(&mut stderr_bytes).await;
            }
        }
    }

    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        return Err(format!("FFmpeg failed: {}", first_lines(&stderr, 5)));
    }
    Ok(())
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn sanitize_ffmpeg_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "")
        .replace('%', "\\%")
}

fn first_lines(text: &str, n: usize) -> String {
    text.lines().take(n).collect::<Vec<_>>().join(" | ")
}

fn is_default_translator(project: &Project) -> bool {
    project.translation_provider.is_empty()
        || project.translation_provider == "marian"
        || project.translation_provider == "argos"
}

fn is_english(language: &str) -> bool {
    language == "English (US)" || language.is_empty()
}

fn resolve_voice(project: &Project, _script: &str) -> String {
    match project.voice_provider.as_str() {
        "edge" => edge_voice_for_language(&project.language).to_string(),
        "piper" => project.voice.clone(),
        "elevenlabs" => match project.voice.as_str() {
            "Amy · English (US)" => "EXAVITQu4vr4xnSDxMaL".to_string(),
            _ => "21m00Tcm4TlvDq8ikWAM".to_string(),
        },
        "openai" => match project.voice.as_str() {
            "Amy · English (US)" => "shimmer".to_string(),
            "Ryan · English (US)" => "onyx".to_string(),
            _ => "alloy".to_string(),
        },
        _ => project.voice.clone(),
    }
}

fn elevenlabs_voice_id(_name: &str) -> String {
    // Default voices - users can map custom voices via Settings later
    "21m00Tcm4TlvDq8ikWAM".to_string()
}

fn read_api_key(provider: &str) -> Result<String, String> {
    let entry = keyring::Entry::new("com.wpgglabs.pdf2vid", provider)
        .map_err(|e| format!("Keyring error: {e}"))?;
    entry
        .get_password()
        .map_err(|_| format!("No API key stored for {}. Open Settings to add one.", provider))
}

/// Synthesize one scene's audio with provider fallback.
///
/// Order of preference (configurable via `project.voiceProvider`):
///   - `edge`     → edge-tts (free, Microsoft Neural, requires network)
///   - `piper`    → local ONNX (offline after model download)
///   - `openai`   → OpenAI TTS (BYO key)
///   - `elevenlabs` → ElevenLabs (BYO key)
///
/// Fallback chain: the user's chosen provider is tried first; if it fails
/// with a recoverable error, the next available provider is attempted.
/// Only hard errors (missing API key, missing model, unimplemented stub)
/// are surfaced immediately — transient failures fall through.
async fn synthesize_scene_audio(
    app: &tauri::AppHandle,
    project: &Project,
    text: &str,
    voice_name: &str,
    audio_path: &std::path::Path,
) -> Result<(), String> {
    let primary = project.voice_provider.as_str();
    let mut attempts: Vec<&str> = vec![primary];

    // Build the fallback chain. Free providers come first so a cloud key
    // error never blocks the free path.
    let mut fallback: Vec<&str> = Vec::new();
    if primary != "edge" && !project.voice_provider.is_empty() {
        fallback.push("edge");
    }
    if primary != "piper" && models::is_model_installed(app, "piper-en_US-amy") {
        fallback.push("piper");
    }
    attempts.extend(fallback);

    let mut last_err: Option<String> = None;
    for provider in attempts {
        match synthesize_with_provider(app, provider, project, text, voice_name, audio_path).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let is_hard = e.starts_with("No API key")
                    || e.contains("is not downloaded")
                    || e.contains("not yet implemented")
                    || e.contains("is required");
                if is_hard && provider == primary {
                    return Err(e);
                }
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "All voice providers failed".into()))
}

async fn synthesize_with_provider(
    app: &tauri::AppHandle,
    provider: &str,
    project: &Project,
    text: &str,
    voice_name: &str,
    audio_path: &std::path::Path,
) -> Result<(), String> {
    let audio_b64 = match provider {
        "edge" => {
            let resp = edgetts::synthesize(edgetts::TtsRequest {
                text: text.to_string(),
                voice: voice_name.to_string(),
                rate: None,
                pitch: None,
            })
            .await?;
            resp.audio_base64
        }
        "piper" => {
            let model_id = "piper-en_US-amy";
            let model_dir = models::model_path(app, model_id)?;
            let resp = cloud::piper_synthesize(
                &model_dir.to_string_lossy(),
                cloud::CloudSynthesisRequest {
                    text: text.to_string(),
                    voice: voice_name.to_string(),
                },
            )
            .await?;
            resp.audio_base64
        }
        "openai" => {
            let key = read_api_key("openai")?;
            let resp = cloud::openai_tts(
                &key,
                cloud::CloudSynthesisRequest {
                    text: text.to_string(),
                    voice: openai_voice_name(project),
                },
            )
            .await?;
            resp.audio_base64
        }
        "elevenlabs" => {
            let key = read_api_key("elevenlabs")?;
            let resp = cloud::elevenlabs_tts(
                &key,
                &elevenlabs_voice_id(voice_name),
                cloud::CloudSynthesisRequest {
                    text: text.to_string(),
                    voice: voice_name.to_string(),
                },
            )
            .await?;
            resp.audio_base64
        }
        other => {
            return Err(format!(
                "Voice provider '{}' is not yet implemented",
                other
            ));
        }
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(audio_b64.as_bytes())
        .map_err(|e| e.to_string())?;
    std::fs::write(audio_path, bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn openai_voice_name(project: &Project) -> String {
    match project.voice.as_str() {
        "Amy · English (US)" => "shimmer".to_string(),
        "Ryan · English (US)" => "onyx".to_string(),
        _ => "alloy".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_chars() {
        assert_eq!(sanitize("hello/world:test"), "hello_world_test");
        assert_eq!(sanitize("My Project 2024"), "My_Project_2024");
    }

    #[test]
    fn is_default_translator_matches() {
        let mut p = make_test_project();
        p.translation_provider = "marian".into();
        assert!(is_default_translator(&p));
        p.translation_provider = "".into();
        assert!(is_default_translator(&p));
        p.translation_provider = "openai".into();
        assert!(!is_default_translator(&p));
    }

    #[test]
    fn is_english_recognizes_us() {
        assert!(is_english("English (US)"));
        assert!(is_english(""));
        assert!(!is_english("Spanish"));
    }

    #[test]
    fn build_ffmpeg_args_shape() {
        let inputs = vec![
            "-loop".into(),
            "1".into(),
            "-i".into(),
            "/tmp/visual.jpg".into(),
            "-i".into(),
            "/tmp/audio.mp3".into(),
        ];
        let filter = "[0:v]scale=1920:1080[vout];[1:a]aresample=44100[aout];concat=n=1:v=1:a=1[vout][aout]".to_string();
        let output = PathBuf::from("/tmp/out.mp4");
        let args = build_ffmpeg_args(&inputs, &filter, &output);

        assert_eq!(args[0], "-y");
        // Input section preserved
        assert!(args.contains(&"/tmp/visual.jpg".to_string()));
        assert!(args.contains(&"/tmp/audio.mp3".to_string()));
        // Filter is quoted as a single arg pair
        let filter_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
        assert_eq!(args[filter_idx + 1], filter);
        // Maps + codecs + output
        assert!(args.windows(2).any(|w| w == ["-map", "[vout]"]));
        assert!(args.windows(2).any(|w| w == ["-map", "[aout]"]));
        assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
        assert!(args.windows(2).any(|w| w == ["-movflags", "+faststart"]));
        // Output is last
        assert_eq!(args.last().unwrap(), "/tmp/out.mp4");
    }

    fn make_test_project() -> Project {
        Project {
            name: "Test".into(),
            source_name: "test.pdf".into(),
            scenes: vec![crate::types::Scene {
                id: "1".into(),
                page: 1,
                title: "Scene".into(),
                script: "Hello world".into(),
                translated_script: None,
                duration: 5,
                selected: true,
                thumbnail: "".into(),
            }],
            language: "Spanish".into(),
            translation_provider: "marian".into(),
            voice_provider: "edge".into(),
            voice: "es-ES-ElviraNeural".into(),
            output_you_tube: true,
            output_tik_tok: false,
        }
    }

    #[test]
    fn drawtext_sanitization() {
        let s = sanitize_ffmpeg_drawtext("It's 100% done: yes");
        assert!(s.contains("\\%"));
        assert!(s.contains("\\:"));
        assert!(!s.contains("'"));
    }
}