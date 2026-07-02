use crate::argos;
use crate::cloud;
use crate::edgetts;
use crate::ffmpeg::{ensure_ffmpeg_or_error, ensure_ffprobe_or_error, Aspect};
use crate::font::{resolve_font, FontRenderKind, FontResolution};
use crate::kokoro;
use crate::models;
use crate::providers::{
    argos_lang_code, edge_voice_for_language, kokoro_lang_code, kokoro_voice_for_language,
};
use crate::state::{cache_dir, AppState};
use crate::types::{
    ExportComplete, ExportError, ExportProgress, ExportRequest, Project, ProjectWarning,
    TranslationWarning, WarningCode,
};
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

    emit_progress(
        &app,
        &request.job_id,
        "Planning",
        "Preparing project",
        2,
        None,
        None,
    );

    let cache = cache_dir(&app)?;
    let audio_dir = cache.join("audio");
    let visuals_dir = cache.join("visuals");
    let render_dir = cache.join("render");
    std::fs::create_dir_all(&audio_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&visuals_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&render_dir).map_err(|e| e.to_string())?;

    // Resolve the drawtext font once for the whole export. We always
    // copy into `render_dir/font.ttf` so the path we hand to FFmpeg has
    // no colons or backslashes — see `font.rs` for the full reasoning.
    let font_resolution = resolve_font(&render_dir);
    let font_warning = font_warning_from(&font_resolution);

    let total = selected.len() as u32;
    let mut current = 0u32;

    // Structured warnings collected throughout the pipeline. The UI
    // reads these as typed records instead of parsing stringly-typed
    // status messages.
    let mut warnings: Vec<ProjectWarning> = Vec::new();
    if let Some(w) = font_warning.clone() {
        warnings.push(w);
    }

    // Stage 1: translate
    let mut translation_warnings: Vec<TranslationWarning> = Vec::new();
    let translated_scenes = if project.translation_provider == "marian"
        || is_default_translator(&project)
    {
        if !is_english(&project.language) {
            emit_progress(
                &app,
                &request.job_id,
                "Translating",
                &format!("Offline Argos to {}", project.language),
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
                let result =
                    argos::translate("en", argos_lang_code(&project.language), &scene.script).await;
                let mut s = scene.clone();
                match result {
                    Ok(translated_text) => s.translated_script = Some(translated_text),
                    Err(e) => {
                        // Argos isn't installed, the language pack is missing,
                        // or the pair is unsupported. Don't pretend it worked:
                        // record a warning and fall back to the source script so
                        // the export still completes (a video, not a hard error).
                        translation_warnings.push(TranslationWarning {
                            scene_id: scene.id.clone(),
                            page: scene.page,
                            provider: "argos".into(),
                            message: format!(
                                "Argos translation unavailable; using source text for page {}",
                                scene.page
                            ),
                        });
                        warnings.push(
                            ProjectWarning::warning(
                                WarningCode::UntranslatedScene,
                                format!("Page {} used the source script: Argos translation was unavailable", scene.page),
                            )
                            .with_scene(scene.id.clone(), scene.page)
                            .with_detail(e.to_string())
                            .with_fix("Install it with `pip install argostranslate`, or switch to OpenAI / Google Cloud in the inspector."),
                        );
                        log::warn!("Argos failed for page {}: {}", scene.page, e);
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
                    warnings.push(
                        ProjectWarning::error(
                            WarningCode::UnsupportedProvider,
                            format!(
                                "Translation provider '{}' is not yet implemented",
                                project.translation_provider
                            ),
                        )
                        .with_scene(scene.id.clone(), scene.page)
                        .with_detail(e.clone()),
                    );
                    return Err(format!(
                        "Translation failed for scene {}: {}",
                        scene.page, e
                    ));
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
    emit_progress(
        &app,
        &request.job_id,
        "Synthesizing",
        "Generating narration",
        30,
        Some(0),
        Some(total),
    );
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

        let result =
            synthesize_scene_audio(&app, &project, &script, &voice_name, &audio_path).await;

        if let Err(e) = result {
            warnings.push(
                ProjectWarning::error(
                    WarningCode::VoiceSynthesisFailed,
                    format!("Voice synthesis failed for page {}: {e}", scene.page),
                )
                .with_scene(scene.id.clone(), scene.page)
                .with_detail(e.clone())
                .with_fix(
                    "Check edge-tts / API key in Settings, or pick a different voice provider.",
                ),
            );
            return Err(e);
        }
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
    emit_progress(
        &app,
        &request.job_id,
        "Visuals",
        "Preparing page visuals",
        65,
        Some(0),
        Some(total),
    );
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
        emit_progress(
            &app,
            &request.job_id,
            "Composing",
            "Rendering YouTube 1920×1080",
            78,
            None,
            None,
        );
        let path = output_dir.join(format!("{}-youtube.mp4", safe_name));
        compose_video(
            state.clone(),
            cancel.clone(),
            &translated_scenes,
            &visual_paths,
            &audio_paths,
            Aspect::Youtube,
            &path,
            &font_resolution,
            &mut warnings,
        )
        .await?;
        youtube_path = Some(path.to_string_lossy().to_string());
    }

    if project.output_tik_tok {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }
        emit_progress(
            &app,
            &request.job_id,
            "Composing",
            "Rendering TikTok 1080×1920",
            90,
            None,
            None,
        );
        let path = output_dir.join(format!("{}-tiktok.mp4", safe_name));
        compose_video(
            state.clone(),
            cancel.clone(),
            &translated_scenes,
            &visual_paths,
            &audio_paths,
            Aspect::Tiktok,
            &path,
            &font_resolution,
            &mut warnings,
        )
        .await?;
        tiktok_path = Some(path.to_string_lossy().to_string());
    }

    emit_progress(
        &app,
        &request.job_id,
        "Done",
        "Export complete",
        100,
        None,
        None,
    );

    let untranslated_count = translation_warnings.len() as u32;
    let render_fallback_used = warnings.iter().any(|w| {
        matches!(
            w.code,
            WarningCode::MissingFont | WarningCode::RenderFallback
        )
    });
    Ok(ExportComplete {
        job_id: request.job_id,
        youtube_path,
        tiktok_path,
        translation_warnings,
        skipped_pages: project.skipped_pages.clone(),
        untranslated_count,
        warnings,
        render_fallback_used,
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

/// Convert a font discovery result into a typed `ProjectWarning`, when
/// one is warranted. A successful resolution produces `None`; a missing
/// font produces a `MissingFont` warning the UI can call out.
fn font_warning_from(resolution: &FontResolution) -> Option<ProjectWarning> {
    if resolution.found {
        return None;
    }
    Some(
        ProjectWarning::warning(
            WarningCode::MissingFont,
            "No drawtext font was found on this system",
        )
        .with_detail(resolution.message.clone())
        .with_fix(
            resolution
                .install_hint
                .clone()
                .unwrap_or_else(|| "Install a TrueType font and re-export.".into()),
        ),
    )
}

// Keep the renderer/import paths from drifting apart by deriving
// `severity` from the resolved font kind. libass fallback would live
// here in Phase 2.5.
#[allow(dead_code)]
fn font_render_kind(resolution: &FontResolution) -> FontRenderKind {
    resolution.render_kind
}

/// Build the FFmpeg argument list for one render. Pure function so the
/// smoke test (and any future test) can verify the filter shape without
/// needing a Tauri runtime.
///
/// `font_path` is the safe render-local path to a TTF that was staged
/// by `font::resolve_font`. When `None` the filter is rendered without
/// `drawtext` — see `font_warning_from` for the corresponding
/// `ProjectWarning` that the caller should surface to the user.
pub fn build_ffmpeg_args(
    inputs: &[String],
    filter: &str,
    output: &std::path::Path,
    font_path: Option<&str>,
) -> Vec<String> {
    let mut args = vec!["-y".to_string()];
    args.extend_from_slice(inputs);
    args.push("-filter_complex".to_string());
    // FFmpeg's option parser treats `:` as a separator. The caller is
    // expected to hand us a path that is already colon-free (the font
    // discovery helper stages under `font.ttf` precisely so this is
    // safe), but we still pass it through `escape_fontfile_for_filter`
    // so a CI override like `--font "C:\Foo\bar.ttf"` cannot break
    // the filter.
    if let Some(p) = font_path {
        let safe = crate::font::escape_fontfile_for_filter(p);
        let rewritten = inject_fontfile_into_filter(filter, &safe);
        args.push(rewritten);
    } else {
        args.push(filter.to_string());
    }
    args.push("-map".to_string());
    args.push("[vout]".to_string());
    args.push("-map".to_string());
    args.push("[aout]".to_string());
    args.push("-c:v".to_string());
    args.push("libx264".to_string());
    args.push("-preset".to_string());
    args.push("medium".to_string());
    args.push("-crf".to_string());
    args.push("20".to_string());
    args.push("-pix_fmt".to_string());
    args.push("yuv420p".to_string());
    args.push("-c:a".to_string());
    args.push("aac".to_string());
    args.push("-b:a".to_string());
    args.push("192k".to_string());
    // `-shortest` bounds the encode to the shorter of the two streams;
    // the looped `-loop 1` image input would otherwise run forever.
    args.push("-shortest".to_string());
    args.push("-movflags".to_string());
    args.push("+faststart".to_string());
    args.push(output.to_string_lossy().to_string());
    args
}

/// Rewrite each `drawtext=text='...'` clause in the filter to also
/// carry `fontfile=<safe>`. The filter is a flat `;`-separated list
/// of filter expressions; we only rewrite the drawtext clauses and
/// leave everything else intact.
fn inject_fontfile_into_filter(filter: &str, safe_fontfile: &str) -> String {
    // `drawtext=` may appear multiple times (one per scene). Replace
    // each occurrence by prefixing with `fontfile=...:`. Other filter
    // expressions never start with `drawtext=`, so a literal replace
    // is safe.
    let needle = "drawtext=";
    let mut out = String::with_capacity(filter.len());
    let mut cursor = 0;
    while let Some(idx) = filter[cursor..].find(needle) {
        let abs = cursor + idx;
        out.push_str(&filter[cursor..abs]);
        out.push_str(&format!("drawtext=fontfile={safe_fontfile}:"));
        cursor = abs + needle.len();
    }
    out.push_str(&filter[cursor..]);
    out
}

/// Run FFmpeg as a cancellable child process. The child handle is
/// registered in AppState so a concurrent `cancel_export` call can
/// terminate it via `start_kill`. The cancel flag is polled between
/// stages as a secondary signal (in case the OS kill races).
#[allow(clippy::too_many_arguments)]
async fn compose_video(
    state: tauri::State<'_, AppState>,
    cancel: Arc<AtomicBool>,
    scenes: &[crate::types::Scene],
    visuals: &[(String, PathBuf)],
    audios: &[(String, PathBuf)],
    aspect: Aspect,
    output: &std::path::Path,
    font: &FontResolution,
    warnings: &mut Vec<ProjectWarning>,
) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }

    let ffmpeg = ensure_ffmpeg_or_error()?;
    let ffprobe = ensure_ffprobe_or_error()?;
    let (w, h) = aspect.dimensions();
    let mut inputs: Vec<String> = Vec::new();
    let mut filter = String::new();
    let mut audio_inputs: Vec<String> = Vec::new();

    // When no font is available we strip drawtext from each scene
    // clause (so the encode still succeeds) and record one
    // `RenderFallback` warning per export. The frontend shows this
    // alongside the structured warnings array.
    let drawtext_available = font.found && font.render_path.is_some();

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

        // The image is fed as a single still frame (no `-loop`): `zoompan`
        // generates the motion frames from it. `-loop` here would stream
        // frames forever and break the concat, so it must stay off.
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
        // Drive the scene length from the real narration audio so the video
        // matches the voice exactly (the stored `duration` is only a
        // word-count estimate). Fall back to it if probing fails.
        let seconds = probe_audio_seconds(&ffprobe, &audio).unwrap_or(scene.duration as f64);
        let frames = ((seconds.max(1.0)) * 25.0).ceil() as u32;

        // Build the premium per-scene video chain (blurred graded
        // background + sharp page + Ken Burns + vignette, with read-along
        // captions that appear line-by-line in time with the narration).
        // Captions are included only when a font was discovered.
        let caption = if drawtext_available {
            Some(script.as_str())
        } else {
            None
        };
        filter.push_str(&build_scene_video_chain(v_idx, i, w, h, frames, caption));
        filter.push_str(&format!(
            "[{a_idx}:a]aresample=44100[a{i}];",
            a_idx = a_idx,
            i = i
        ));
        audio_inputs.push(format!("[v{i}][a{i}]", i = i));
    }

    let concat_inputs = audio_inputs.join("");
    filter.push_str(&format!(
        "{concat_inputs}concat=n={n}:v=1:a=1[vout][aout]",
        concat_inputs = concat_inputs,
        n = audio_inputs.len()
    ));

    let font_arg = if drawtext_available {
        font.render_path.as_deref()
    } else {
        None
    };
    let args = build_ffmpeg_args(&inputs, &filter, std::path::Path::new(&output), font_arg);

    if !drawtext_available {
        // Surface this once per export — the per-scene fallback would
        // spam the warnings list without adding information.
        if !warnings
            .iter()
            .any(|w| matches!(w.code, WarningCode::RenderFallback))
        {
            warnings.push(
                ProjectWarning::warning(
                    WarningCode::RenderFallback,
                    "No drawtext font found. Videos will render without on-screen narration.",
                )
                .with_detail(font.message.clone())
                .with_fix(
                    font.install_hint
                        .clone()
                        .unwrap_or_else(|| "Install a TrueType font and re-export.".into()),
                ),
            );
        }
    }

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
    state.replace_ffmpeg_child_inner(child_slot.clone()).await;

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg: {e}"))?;

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

/// Probe the duration (in seconds) of an audio file via ffprobe. Returns
/// `None` if ffprobe fails or the output can't be parsed, so the caller
/// can fall back to the stored scene estimate.
fn probe_audio_seconds(ffprobe: &std::path::Path, audio: &std::path::Path) -> Option<f64> {
    let mut cmd = std::process::Command::new(ffprobe);
    cmd.args([
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
    ])
    .arg(audio)
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let val = s.trim().parse::<f64>().ok()?;
    if val.is_finite() && val > 0.0 {
        Some(val)
    } else {
        None
    }
}

/// Build the premium per-scene video filter chain for scene index `i`,
/// reading from input pad `v_idx` and emitting the labelled output
/// `[v{i}]`.
///
/// The look:
///   - a blurred, slightly darkened + saturated copy of the page fills
///     the frame (no black letterbox bars),
///   - the sharp page is composited centered on top,
///   - a slow Ken Burns zoom + vignette add cinematic motion,
///   - an optional drop-shadow caption is drawn when a font is present.
///
/// This is a pure function so the exact filter shape can be unit tested;
/// the graph itself is validated by rendering with a real ffmpeg.
fn build_scene_video_chain(
    v_idx: usize,
    i: usize,
    w: u32,
    h: u32,
    frames: u32,
    caption: Option<&str>,
) -> String {
    // The image is fed to `zoompan` as a single still frame (the input is
    // NOT `-loop`ed), so `d` is the total number of output frames for the
    // scene. `fps=25` pins the output rate; `setsar=1` + `format` keep
    // every scene concat-compatible.
    let mut chain = format!(
        "[{v_idx}:v]split=2[bg{i}][fg{i}];\
[bg{i}]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},boxblur=24:2,eq=brightness=-0.12:saturation=1.15[bgb{i}];\
[fg{i}]scale={w}:{h}:force_original_aspect_ratio=decrease[fgs{i}];\
[bgb{i}][fgs{i}]overlay=(W-w)/2:(H-h)/2,zoompan=z='min(zoom+0.0008,1.15)':d={frames}:s={w}x{h}:fps=25,vignette=PI/5,setsar=1,fps=25,format=yuv420p"
    );
    if let Some(script) = caption {
        // Read-along captions: the script is wrapped into short lines and
        // each line is shown only during its slice of the scene, so the
        // on-screen text follows the narration line by line. Timing is
        // proportional to line length across the audio-derived frame count.
        let lines = split_caption_lines(script, 42);
        for (line, start_frame, end_frame) in timed_caption_lines(&lines, frames) {
            let safe = sanitize_ffmpeg_drawtext(&line);
            let start = start_frame as f64 / 25.0;
            let end = end_frame as f64 / 25.0;
            chain.push_str(&format!(
                ",drawtext=text='{safe}':fontcolor=white:fontsize=44:shadowcolor=black@0.8:shadowx=2:shadowy=2:box=1:boxcolor=black@0.45:boxborderw=18:x=(w-text_w)/2:y=h-90:enable='between(t,{start:.2},{end:.2})'"
            ));
        }
    }
    chain.push_str(&format!("[v{i}];"));
    chain
}

/// Word-wrap caption text into short lines (roughly `max_chars` each) so
/// captions display a line at a time rather than a wall of text.
fn split_caption_lines(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > max_chars {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Distribute `frames` across the caption `lines` proportional to each
/// line's length, producing `(line, start_frame, end_frame)` windows that
/// are contiguous, gapless, and together cover the whole scene.
fn timed_caption_lines(lines: &[String], frames: u32) -> Vec<(String, u32, u32)> {
    if lines.is_empty() || frames == 0 {
        return Vec::new();
    }
    let total: u64 = lines.iter().map(|l| l.chars().count().max(1) as u64).sum();
    let mut out = Vec::with_capacity(lines.len());
    let mut acc = 0u32;
    let last = lines.len() - 1;
    for (idx, line) in lines.iter().enumerate() {
        let weight = line.chars().count().max(1) as u64;
        let end = if idx == last {
            frames
        } else {
            (acc as u64 + weight * frames as u64 / total).min(frames as u64) as u32
        };
        let end = end.max(acc + 1).min(frames);
        out.push((line.clone(), acc, end));
        acc = end;
    }
    out
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
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

/// Convert a narration speed percentage (100 = normal) into an edge-tts
/// `--rate` string like `+15%` / `-20%`. Returns `None` at 100% so we
/// don't pass a no-op flag. The UI bounds speed to 75–125, but we clamp
/// defensively in case a stale project carries an out-of-range value.
pub(crate) fn speed_to_rate(speed: u32) -> Option<String> {
    let clamped = speed.clamp(50, 200) as i32;
    let delta = clamped - 100;
    if delta == 0 {
        None
    } else if delta > 0 {
        Some(format!("+{delta}%"))
    } else {
        Some(format!("{delta}%"))
    }
}

fn resolve_voice(project: &Project, _script: &str) -> String {
    match project.voice_provider.as_str() {
        "edge" => edge_voice_for_language(&project.language).to_string(),
        // A kokoro voice id contains an underscore (e.g. `af_heart`); if the
        // stored voice isn't one, pick a sensible default for the language.
        "kokoro" => {
            if project.voice.contains('_') {
                project.voice.clone()
            } else {
                kokoro_voice_for_language(&project.language).to_string()
            }
        }
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
    entry.get_password().map_err(|_| {
        format!(
            "No API key stored for {}. Open Settings to add one.",
            provider
        )
    })
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
                rate: speed_to_rate(project.voice_speed),
                pitch: None,
            })
            .await?;
            resp.audio_base64
        }
        "kokoro" => {
            let resp = kokoro::synthesize(kokoro::KokoroRequest {
                text: text.to_string(),
                voice: voice_name.to_string(),
                lang_code: kokoro_lang_code(&project.language).to_string(),
                speed: project.voice_speed as f32 / 100.0,
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
            return Err(format!("Voice provider '{}' is not yet implemented", other));
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
    use crate::types::WarningSeverity;

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
    fn scene_chain_has_premium_elements() {
        let chain = build_scene_video_chain(0, 0, 1920, 1080, 100, Some("hello"));
        // Blurred, graded background instead of black letterbox bars.
        assert!(chain.contains("split=2[bg0][fg0]"));
        assert!(chain.contains("boxblur"));
        assert!(chain.contains("overlay=(W-w)/2:(H-h)/2"));
        // Frame count is passed straight through to zoompan's `d`.
        assert!(chain.contains("zoompan"));
        assert!(chain.contains("d=100:"));
        assert!(chain.contains("vignette"));
        // Caption carries a drop shadow and a read-along enable window.
        assert!(chain.contains("drawtext=text='hello'"));
        assert!(chain.contains("shadowx=2"));
        assert!(chain.contains("enable='between(t,"));
        // No black pad bars.
        assert!(!chain.contains(":black"));
        // Emits the expected labelled output.
        assert!(chain.ends_with("[v0];"));
    }

    #[test]
    fn split_caption_lines_wraps_on_word_boundaries() {
        let lines = split_caption_lines("the quick brown fox jumps over the lazy dog", 15);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| l.chars().count() <= 15));
        // No word is split across lines.
        assert_eq!(
            lines.join(" "),
            "the quick brown fox jumps over the lazy dog"
        );
    }

    #[test]
    fn timed_caption_lines_cover_scene_without_gaps() {
        let lines = vec![
            "one".to_string(),
            "two two".to_string(),
            "three".to_string(),
        ];
        let timed = timed_caption_lines(&lines, 100);
        // First starts at 0, last ends exactly at the frame count.
        assert_eq!(timed.first().unwrap().1, 0);
        assert_eq!(timed.last().unwrap().2, 100);
        // Windows are contiguous and strictly increasing.
        for pair in timed.windows(2) {
            assert_eq!(pair[0].2, pair[1].1);
            assert!(pair[0].1 < pair[0].2);
        }
    }

    #[test]
    fn read_along_emits_one_drawtext_per_line() {
        // A long script wraps to multiple lines => multiple timed drawtexts.
        let script = "This is a reasonably long narration sentence that should wrap \
                      across several caption lines when rendered on screen.";
        let chain = build_scene_video_chain(0, 0, 1920, 1080, 250, Some(script));
        let count = chain.matches("drawtext=text=").count();
        assert!(count >= 2, "expected multiple caption lines, got {count}");
        assert_eq!(chain.matches("enable='between(t,").count(), count);
    }

    #[test]
    fn scene_chain_without_font_omits_drawtext() {
        let chain = build_scene_video_chain(2, 1, 1080, 1920, 150, None);
        assert!(chain.contains("[2:v]split=2[bg1][fg1]"));
        assert!(chain.contains("d=150:"));
        assert!(!chain.contains("drawtext"));
        assert!(chain.ends_with("[v1];"));
    }

    #[test]
    fn speed_to_rate_maps_percentages() {
        assert_eq!(speed_to_rate(100), None);
        assert_eq!(speed_to_rate(125), Some("+25%".to_string()));
        assert_eq!(speed_to_rate(75), Some("-25%".to_string()));
        // Clamps defensively out-of-range values.
        assert_eq!(speed_to_rate(0), Some("-50%".to_string()));
        assert_eq!(speed_to_rate(999), Some("+100%".to_string()));
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
        let filter =
            "[0:v]scale=1920:1080[vout];[1:a]aresample=44100[aout];concat=n=1:v=1:a=1[vout][aout]"
                .to_string();
        let output = PathBuf::from("/tmp/out.mp4");
        let args = build_ffmpeg_args(&inputs, &filter, &output, None);

        assert_eq!(args[0], "-y");
        // Input section preserved
        assert!(args.contains(&"/tmp/visual.jpg".to_string()));
        assert!(args.contains(&"/tmp/audio.mp3".to_string()));
        // Filter is quoted as a single arg pair
        let filter_idx = args
            .iter()
            .position(|a| a == "-filter_complex")
            .expect("ffmpeg args should include -filter_complex");
        assert_eq!(args[filter_idx + 1], filter);
        // Maps + codecs + output
        assert!(args.windows(2).any(|w| w == ["-map", "[vout]"]));
        assert!(args.windows(2).any(|w| w == ["-map", "[aout]"]));
        assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
        assert!(args.windows(2).any(|w| w == ["-movflags", "+faststart"]));
        assert!(args.contains(&"-shortest".to_string()));
        // Output is last
        assert_eq!(args.last().unwrap(), "/tmp/out.mp4");
    }

    #[test]
    fn build_ffmpeg_args_injects_fontfile() {
        let inputs = vec![
            "-loop".into(),
            "1".into(),
            "-i".into(),
            "/tmp/visual.jpg".into(),
            "-i".into(),
            "/tmp/audio.mp3".into(),
        ];
        let filter = "[0:v]scale=1920:1080,drawtext=text='hi':x=10[v0];[1:a]aresample=44100[a0];[v0][a0]concat=n=1:v=1:a=1[vout][aout]".to_string();
        let output = PathBuf::from("/tmp/out.mp4");
        let args = build_ffmpeg_args(&inputs, &filter, &output, Some("/tmp/font.ttf"));
        let filter_idx = args
            .iter()
            .position(|a| a == "-filter_complex")
            .expect("ffmpeg args should include -filter_complex");
        let injected = &args[filter_idx + 1];
        // drawtext must now carry fontfile.
        assert!(injected.contains("drawtext=fontfile=/tmp/font.ttf:text='hi'"));
        // Original drawtext= prefix should be replaced, not duplicated.
        assert!(!injected.contains("drawtext=text"));
    }

    #[test]
    fn build_ffmpeg_args_injects_fontfile_into_multiple_drawtext_clauses() {
        let inputs = vec![
            "-i".into(),
            "/tmp/a.jpg".into(),
            "-i".into(),
            "/tmp/b.jpg".into(),
        ];
        let filter = "[0:v]scale=10:10,drawtext=text='one':x=0[v0];[1:v]scale=10:10,drawtext=text='two':x=0[v1];[v0][v1]concat=n=2:v=1:a=0[vout]".to_string();
        let args = build_ffmpeg_args(
            &inputs,
            &filter,
            &PathBuf::from("/tmp/out.mp4"),
            Some("/tmp/font.ttf"),
        );
        let filter_idx = args
            .iter()
            .position(|a| a == "-filter_complex")
            .expect("ffmpeg args should include -filter_complex");
        let injected = &args[filter_idx + 1];
        assert_eq!(
            injected.matches("drawtext=fontfile=/tmp/font.ttf").count(),
            2
        );
    }

    #[test]
    fn build_ffmpeg_args_with_windows_style_font_path_escapes() {
        let inputs = vec!["-i".into(), "/tmp/visual.jpg".into()];
        let filter = "[0:v]scale=10:10,drawtext=text='hi'[vout]".to_string();
        let args = build_ffmpeg_args(
            &inputs,
            &filter,
            &PathBuf::from("/tmp/out.mp4"),
            Some(r"C:\Windows\Fonts\arial.ttf"),
        );
        let filter_idx = args
            .iter()
            .position(|a| a == "-filter_complex")
            .expect("ffmpeg args should include -filter_complex");
        let injected = &args[filter_idx + 1];
        // The colon in C:\ must be escaped, and backslashes too.
        assert!(injected.contains(r"fontfile=C\:\\Windows\\Fonts\\arial.ttf"));
    }

    #[test]
    fn build_ffmpeg_args_missing_font_keeps_filter_intact() {
        let inputs = vec!["-i".into(), "/tmp/visual.jpg".into()];
        let filter = "[0:v]scale=10:10,drawtext=text='hi'[vout]".to_string();
        let args = build_ffmpeg_args(&inputs, &filter, &PathBuf::from("/tmp/out.mp4"), None);
        let filter_idx = args
            .iter()
            .position(|a| a == "-filter_complex")
            .expect("ffmpeg args should include -filter_complex");
        assert_eq!(args[filter_idx + 1], filter);
    }

    #[test]
    fn font_warning_from_returns_none_when_found() {
        let r = FontResolution {
            found: true,
            source_path: Some("/tmp/a.ttf".into()),
            render_path: Some("/tmp/font.ttf".into()),
            render_kind: FontRenderKind::Workdir,
            message: "ok".into(),
            install_hint: None,
        };
        assert!(font_warning_from(&r).is_none());
    }

    #[test]
    fn font_warning_from_populates_missing_font() {
        let r = FontResolution {
            found: false,
            source_path: None,
            render_path: None,
            render_kind: FontRenderKind::None,
            message: "no fonts".into(),
            install_hint: Some("install dejavu".into()),
        };
        let w = font_warning_from(&r).unwrap();
        assert_eq!(w.code, WarningCode::MissingFont);
        assert_eq!(w.severity, WarningSeverity::Warning);
        assert!(w.detail.is_some());
        assert!(w.suggested_fix.is_some());
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
            skipped_pages: vec![],
            voice_speed: 100,
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
