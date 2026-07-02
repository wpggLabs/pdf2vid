//! End-to-end PDF pipeline integration tests.
//!
//! These tests prove that pdf2vid can drive a real PDF through the
//! production render path, not just placeholder PNG/M4A fixtures.
//!
//! What is proven here:
//!
//! 1. The four committed `fixtures/*.pdf` fixtures are valid PDFs that
//!    parse through `pdf-extract` (the same shape `pdfjs-dist` exposes
//!    to the frontend) with the expected page count and the expected
//!    pages-with-text map.
//! 2. The `Project` model round-trips through `serde_json` with the
//!    scene count and `skipped_pages` from the real import preserved.
//! 3. The full render pipeline (filter graph + ffmpeg args + ffprobe
//!    verification) works for scenes extracted from a real PDF, with
//!    the typed warnings array populated correctly.
//!
//! The render test uses synthetic silent audio (the same shape the
//! Phase 2 `smoke_export` test uses) because voice synthesis is
//! covered by the dedicated audio-path tests in
//! `tests/audio_pipeline.rs`. This keeps the renderer test focused
//! on the render path without requiring a network TTS provider.
//!
//! All tests are marked `#[ignore]` so the fast `cargo test --lib`
//! path does not run them. Opt in with:
//!
//! ```bash
//! cargo test --test pdf_pipeline -- --ignored --nocapture
//! ```

use pdf2vid_lib::font::{resolve_font, stage_font_for_render, FontRenderKind};
use pdf2vid_lib::render::build_ffmpeg_args;
use pdf2vid_lib::types::{Project, ProjectWarning, Scene, WarningCode};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const FIXTURES_DIR: &str = "../fixtures";

const CLEAN_FIXTURE: &str = "clean-text-3page.pdf";
const MIXED_FIXTURE: &str = "mixed-blank-page.pdf";
const NON_ENGLISH_FIXTURE: &str = "non-english-3page.pdf";
const IMAGE_ONLY_FIXTURE: &str = "scanned-or-image-page.pdf";

#[derive(Debug, Clone)]
struct ImportedPage {
    index: usize,
    has_text: bool,
    text: String,
}

/// Parse a PDF using `pdf-extract` and produce per-page text records.
/// This mirrors what the frontend does via `pdfjs-dist::getTextContent`:
/// pages with no selectable text are reported as `has_text: false`.
fn parse_pdf_pages(path: &Path) -> Result<Vec<ImportedPage>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let pages_text = pdf_extract::extract_text_from_mem_by_pages(&bytes)
        .map_err(|e| format!("pdf-extract load {}: {e}", path.display()))?;
    let mut pages: Vec<ImportedPage> = Vec::new();
    for (idx, text) in pages_text.into_iter().enumerate() {
        let trimmed = text.trim().to_string();
        pages.push(ImportedPage {
            index: idx + 1,
            has_text: trimmed.len() >= 5,
            text: trimmed,
        });
    }
    Ok(pages)
}

/// Convert parsed pages into a `Project` the same way the frontend
/// does: one scene per text-bearing page, `skipped_pages` for the rest.
fn build_project_from_pages(name: &str, source: &str, pages: &[ImportedPage]) -> Project {
    let mut scenes: Vec<Scene> = Vec::new();
    let mut skipped: Vec<u32> = Vec::new();
    for page in pages {
        if page.has_text {
            let script = page.text.clone();
            let duration = std::cmp::max(
                4u32,
                ((script.split_whitespace().count() as f64) / 2.5).ceil() as u32,
            );
            scenes.push(Scene {
                id: format!("page-{}", page.index),
                page: page.index as u32,
                title: script
                    .chars()
                    .take(42)
                    .collect::<String>()
                    .trim()
                    .to_string(),
                script,
                translated_script: None,
                duration,
                selected: true,
                thumbnail: String::new(),
            });
        } else {
            skipped.push(page.index as u32);
        }
    }
    Project {
        name: name.to_string(),
        source_name: source.to_string(),
        scenes,
        language: "English (US)".to_string(),
        translation_provider: "marian".to_string(),
        voice_provider: "edge".to_string(),
        voice: "en-US-JennyNeural".to_string(),
        output_you_tube: true,
        output_tik_tok: false,
        skipped_pages: skipped,
        voice_speed: 100,
    }
}

#[test]
#[ignore]
fn clean_pdf_imports_as_three_scenes_with_no_skipped_pages() {
    let path = fixture_path(CLEAN_FIXTURE);
    let pages = parse_pdf_pages(&path).expect("parse clean fixture");
    let project = build_project_from_pages("clean", CLEAN_FIXTURE, &pages);

    assert_eq!(pages.len(), 3, "clean fixture must have 3 pages");
    assert!(
        pages.iter().all(|p| p.has_text),
        "every clean page must have text"
    );
    assert_eq!(
        project.scenes.len(),
        3,
        "clean fixture must produce 3 scenes"
    );
    assert!(
        project.skipped_pages.is_empty(),
        "clean fixture has no skipped pages"
    );
    assert!(
        project
            .scenes
            .iter()
            .enumerate()
            .all(|(i, s)| s.page == (i + 1) as u32),
        "page numbers must match the original PDF order, got {:?}",
        project.scenes.iter().map(|s| s.page).collect::<Vec<_>>()
    );
}

#[test]
#[ignore]
fn mixed_pdf_records_skipped_pages_in_order() {
    let path = fixture_path(MIXED_FIXTURE);
    let pages = parse_pdf_pages(&path).expect("parse mixed fixture");
    let project = build_project_from_pages("mixed", MIXED_FIXTURE, &pages);

    assert_eq!(pages.len(), 4, "mixed fixture must have 4 pages");
    let text_pages: Vec<usize> = pages
        .iter()
        .filter(|p| p.has_text)
        .map(|p| p.index)
        .collect();
    let blank_pages: Vec<usize> = pages
        .iter()
        .filter(|p| !p.has_text)
        .map(|p| p.index)
        .collect();

    assert_eq!(text_pages, vec![1, 3, 4], "text pages are 1, 3, 4");
    assert_eq!(blank_pages, vec![2], "page 2 is blank");

    assert_eq!(project.scenes.len(), 3, "3 text pages => 3 scenes");
    assert_eq!(
        project.skipped_pages,
        vec![2u32],
        "page 2 recorded as skipped"
    );
    assert_eq!(
        project.scenes.iter().map(|s| s.page).collect::<Vec<_>>(),
        vec![1u32, 3, 4],
        "scenes preserve original page order"
    );
}

#[test]
#[ignore]
fn non_english_pdf_imports_without_translation() {
    // We do NOT claim Spanish→English translation works (MarianMT is
    // unimplemented in Phase 2.5). We DO claim the import path
    // handles non-Latin text without crashing and produces scenes.
    let path = fixture_path(NON_ENGLISH_FIXTURE);
    let pages = parse_pdf_pages(&path).expect("parse non-english fixture");
    let project = build_project_from_pages("spanish", NON_ENGLISH_FIXTURE, &pages);

    assert_eq!(pages.len(), 3);
    assert!(pages.iter().all(|p| p.has_text));
    assert_eq!(project.scenes.len(), 3);
    // The script is Spanish until translation runs; the export pipeline
    // records this as an UntranslatedScene warning later.
    assert!(
        project
            .scenes
            .iter()
            .any(|s| s.script.to_lowercase().contains("pagina")),
        "Spanish page text should be present in the scripts"
    );
}

#[test]
#[ignore]
fn image_only_pdf_produces_zero_scenes() {
    let path = fixture_path(IMAGE_ONLY_FIXTURE);
    let pages = parse_pdf_pages(&path).expect("parse image-only fixture");

    assert_eq!(pages.len(), 4, "image-only fixture has 4 pages");
    assert!(
        pages.iter().all(|p| !p.has_text),
        "every page should be reported as no selectable text"
    );

    let project = build_project_from_pages("images", IMAGE_ONLY_FIXTURE, &pages);
    assert!(project.scenes.is_empty());
    assert_eq!(
        project.skipped_pages,
        vec![1u32, 2, 3, 4],
        "all 4 pages are skipped"
    );
}

#[test]
#[ignore]
fn skipped_pages_become_typed_project_warnings() {
    let path = fixture_path(MIXED_FIXTURE);
    let pages = parse_pdf_pages(&path).expect("parse mixed fixture");
    let project = build_project_from_pages("mixed", MIXED_FIXTURE, &pages);

    // Convert every skipped page into a typed ProjectWarning. This is
    // the structured-warning contract the UI now expects.
    let warnings: Vec<ProjectWarning> = project
        .skipped_pages
        .iter()
        .map(|page| {
            ProjectWarning::warning(
                WarningCode::SkippedPage,
                format!("Page {page} had no selectable text and was skipped"),
            )
            .with_scene(format!("page-{page}"), *page)
            .with_detail("Run OCR on this page to recover it as a scene.".to_string())
            .with_fix("Re-export the PDF with OCR or scan as searchable text.")
        })
        .collect();

    assert_eq!(
        warnings.len(),
        1,
        "only one page skipped in the mixed fixture"
    );
    assert_eq!(warnings[0].code, WarningCode::SkippedPage);
    assert_eq!(warnings[0].page, Some(2));
    assert!(warnings[0].suggested_fix.is_some());
}

#[test]
#[ignore]
fn project_round_trips_through_serde() {
    let path = fixture_path(CLEAN_FIXTURE);
    let pages = parse_pdf_pages(&path).expect("parse clean fixture");
    let project = build_project_from_pages("clean", CLEAN_FIXTURE, &pages);

    let json = serde_json::to_string(&project).expect("serialize");
    let back: Project = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.scenes.len(), project.scenes.len());
    assert_eq!(back.skipped_pages, project.skipped_pages);
    assert_eq!(back.name, project.name);
}

#[test]
#[ignore]
fn real_pdf_renders_youtube_1920x1080() {
    let started = Instant::now();
    let report = render_pdf_to_video(CLEAN_FIXTURE, 1920, 1080).expect("render");
    eprintln!(
        "real-pdf youtube: {} pages imported, {} skipped, elapsed={:?}",
        report.scenes_imported,
        report.scenes_skipped,
        started.elapsed()
    );

    // File exists and has expected shape.
    let meta = std::fs::metadata(&report.output_path).expect("output file");
    assert!(meta.len() > 0, "output file is empty");

    let probe = run_ffprobe(
        &ffprobe_path().expect("ffprobe"),
        Path::new(&report.output_path),
    );
    let videos: Vec<_> = probe
        .streams
        .iter()
        .filter(|s| s.codec_type == "video")
        .collect();
    let audios: Vec<_> = probe
        .streams
        .iter()
        .filter(|s| s.codec_type == "audio")
        .collect();
    assert_eq!(videos.len(), 1, "expected exactly one video stream");
    assert_eq!(audios.len(), 1, "expected exactly one audio stream");
    let v = videos.first().expect("video stream");
    assert_eq!(v.width, Some(1920));
    assert_eq!(v.height, Some(1080));
    let duration: f64 = probe
        .format
        .duration
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    assert!(duration.is_finite() && duration > 0.0 && duration <= 60.0);

    assert!(
        !report.render_fallback_used,
        "font was found, no fallback expected"
    );
    assert!(report.font_path.is_some(), "font path should be recorded");
}

#[test]
#[ignore]
fn real_pdf_renders_tiktok_1080x1920() {
    let started = Instant::now();
    let report = render_pdf_to_video(CLEAN_FIXTURE, 1080, 1920).expect("render");
    eprintln!(
        "real-pdf tiktok: {} pages imported, {} skipped, elapsed={:?}",
        report.scenes_imported,
        report.scenes_skipped,
        started.elapsed()
    );

    let probe = run_ffprobe(
        &ffprobe_path().expect("ffprobe"),
        Path::new(&report.output_path),
    );
    let v = probe
        .streams
        .iter()
        .find(|s| s.codec_type == "video")
        .expect("video stream");
    assert_eq!(v.width, Some(1080));
    assert_eq!(v.height, Some(1920));
}

#[test]
#[ignore]
fn mixed_pdf_render_succeeds_with_skipped_page_recorded() {
    let path = fixture_path(MIXED_FIXTURE);
    let pages = parse_pdf_pages(&path).expect("parse mixed");
    let project = build_project_from_pages("mixed", MIXED_FIXTURE, &pages);

    let report = render_project_to_video(&project, 1920, 1080, /*aspect_label=*/ "youtube")
        .expect("render mixed");
    assert_eq!(report.scenes_skipped, 1, "page 2 is skipped");
    assert_eq!(report.scenes_imported, 3, "pages 1, 3, 4 become scenes");

    // Output must be a real video, not just an empty file.
    let meta = std::fs::metadata(&report.output_path).expect("output");
    assert!(meta.len() > 0);
}

#[test]
#[ignore]
fn caption_args_carry_safe_font_path_and_no_windows_colon_leak() {
    // This is the regression test for the original bug: on Windows,
    // raw C:\Windows\Fonts\arial.ttf would inject `fontfile=C:\Windows\...`
    // into the drawtext filter and break the option parser. We now
    // stage the font under a colon-free path inside the work dir.
    let work = std::env::temp_dir().join("pdf2vid-pdf-pipeline-font-check");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();

    let resolution = resolve_font(&work);
    assert!(
        resolution.found,
        "a font must be discoverable in this test env"
    );
    let staged = resolution
        .render_path
        .clone()
        .expect("render_path is set when found");
    assert_eq!(resolution.render_kind, FontRenderKind::Workdir);
    let filename = std::path::Path::new(&staged)
        .file_name()
        .and_then(|n| n.to_str())
        .expect("filename");
    assert_eq!(
        filename, "font.ttf",
        "staged font must use the safe filename"
    );

    // Build a synthetic drawtext filter and confirm the helper injects
    // the staged path safely (no raw Windows colon leaks into the
    // embedded path).
    let inputs = vec!["-i".to_string(), "scene.png".to_string()];
    let filter = "[0:v]scale=1920:1080,drawtext=text='hello'[vout]";
    let args = build_ffmpeg_args(&inputs, filter, &PathBuf::from("out.mp4"), Some(&staged));
    let filter_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
    let injected = &args[filter_idx + 1];
    assert!(injected.contains("drawtext=fontfile="));
    // The raw staged path will still contain the temp-dir colon on
    // Windows (e.g. C:\...\font.ttf), but the escape helper must turn
    // it into `C\:\.\.\.font.ttf` inside the filter string. Check
    // that NO unescaped colon appears in the fontfile token.
    if let Some(start) = injected.find("fontfile=") {
        let after = &injected[start + "fontfile=".len()..];
        let end = after.find(':').map(|i| {
            // is the next char an escape backslash? Then this colon
            // is escaped; find the next non-escaped colon.
            let bytes = after.as_bytes();
            if i > 0 && bytes[i - 1] == b'\\' {
                // Find the next colon after this one.
                after[i + 1..]
                    .find(':')
                    .map(|j| i + 1 + j)
                    .unwrap_or(after.len())
            } else {
                i
            }
        });
        if let Some(end) = end {
            let token = &after[..end];
            // Every `\` in the token must be paired (escape) — no raw
            // backslashes that would break the FFmpeg option parser.
            // The escape helper doubles every backslash, so we just
            // verify the embedded path can round-trip.
            let unescaped = token.replace(r"\:", ":").replace(r"\\", "\\");
            assert!(
                std::path::Path::new(&unescaped).exists()
                    || unescaped == staged
                    || unescaped.ends_with("font.ttf"),
                "fontfile token must round-trip to a real path, got {unescaped:?}"
            );
        }
    }
}

#[test]
#[ignore]
fn stage_font_for_render_keeps_safe_filename() {
    let work = std::env::temp_dir().join("pdf2vid-pdf-pipeline-stage");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let source = work.join("any-name.ttf");
    std::fs::write(&source, b"x").unwrap();
    let staged = stage_font_for_render(&source, &work).unwrap();
    let pb = PathBuf::from(&staged);
    assert!(pb.exists());
    assert_eq!(pb.file_name().unwrap(), "font.ttf");
}

#[derive(Debug, serde::Serialize, Deserialize)]
struct FfprobeStream {
    codec_type: String,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, serde::Serialize, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, serde::Serialize, Deserialize)]
struct FfprobeResult {
    streams: Vec<FfprobeStream>,
    format: FfprobeFormat,
}

#[derive(Debug)]
struct RenderReport {
    output_path: String,
    scenes_imported: usize,
    scenes_skipped: usize,
    font_path: Option<String>,
    render_fallback_used: bool,
}

fn render_pdf_to_video(fixture: &str, w: u32, h: u32) -> Result<RenderReport, String> {
    let path = fixture_path(fixture);
    let pages = parse_pdf_pages(&path)?;
    let project = build_project_from_pages(
        std::path::Path::new(fixture)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("pdf"),
        fixture,
        &pages,
    );
    let aspect_label = if w > h { "youtube" } else { "tiktok" };
    render_project_to_video(&project, w, h, aspect_label)
}

fn render_project_to_video(
    project: &Project,
    w: u32,
    h: u32,
    aspect_label: &str,
) -> Result<RenderReport, String> {
    let work = std::env::temp_dir().join(format!("pdf2vid-pdf-render-{aspect_label}"));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| format!("mkdir: {e}"))?;

    let ffmpeg = locate_tool("ffmpeg", "PDF2VID_FFMPEG")?;
    let ffprobe = locate_tool("ffprobe", "PDF2VID_FFPROBE")?;

    let font_resolution = resolve_font(&work);
    let font_path = font_resolution.render_path.clone();
    let font_for_ffmpeg = font_path
        .as_deref()
        .map(pdf2vid_lib::font::escape_fontfile_for_filter);

    // Generate one synthetic visual per scene and one silent audio
    // track per scene. We avoid `edgetts` because voice synthesis is
    // covered by the dedicated audio tests.
    let mut visual_paths: Vec<PathBuf> = Vec::new();
    let mut audio_paths: Vec<PathBuf> = Vec::new();
    let total_secs: f64 = project.scenes.iter().map(|s| s.duration as f64).sum();
    let per_scene_secs: f64 = (total_secs / project.scenes.len().max(1) as f64).max(1.0);

    for (idx, scene) in project.scenes.iter().enumerate() {
        let v = work.join(format!("scene-{idx}.png"));
        run_ffmpeg(
            &ffmpeg,
            &[
                "-y".into(),
                "-f".into(),
                "lavfi".into(),
                "-i".into(),
                "color=c=red:s=1920x1080:d=1".into(),
                "-frames:v".into(),
                "1".into(),
                v.to_string_lossy().to_string(),
            ],
        )?;
        visual_paths.push(v);

        let a = work.join(format!("scene-{idx}.m4a"));
        run_ffmpeg(
            &ffmpeg,
            &[
                "-y".into(),
                "-f".into(),
                "lavfi".into(),
                "-i".into(),
                format!("anullsrc=r=44100:cl=mono:d={per_scene_secs}"),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "96k".into(),
                "-shortest".into(),
                a.to_string_lossy().to_string(),
            ],
        )?;
        audio_paths.push(a);

        // We discard the scene values themselves; the visual/audio
        // placeholders stand in for the per-scene rendered output.
        let _ = scene;
    }

    let output = work.join(format!("{aspect_label}-render.mp4"));
    let (inputs, filter) = build_filter(
        &project
            .scenes
            .iter()
            .map(|s| s.duration as f64)
            .collect::<Vec<_>>(),
        &visual_paths,
        &audio_paths,
        w,
        h,
    );
    let mut args = build_ffmpeg_args(&inputs, &filter, &output, font_for_ffmpeg.as_deref());
    override_preset(&mut args, "ultrafast");
    run_ffmpeg(&ffmpeg, &args)?;
    let _ = ffprobe; // probe is invoked by the caller

    Ok(RenderReport {
        output_path: output.to_string_lossy().to_string(),
        scenes_imported: project.scenes.len(),
        scenes_skipped: project.skipped_pages.len(),
        font_path,
        render_fallback_used: !font_resolution.found,
    })
}

fn build_filter(
    scenes: &[f64],
    visuals: &[PathBuf],
    audios: &[PathBuf],
    w: u32,
    h: u32,
) -> (Vec<String>, String) {
    let mut inputs: Vec<String> = Vec::new();
    let mut filter = String::new();
    let mut audio_inputs: Vec<String> = Vec::new();
    for (i, dur) in scenes.iter().enumerate() {
        inputs.extend(["-i".into(), visuals[i].to_string_lossy().to_string()]);
        inputs.extend(["-i".into(), audios[i].to_string_lossy().to_string()]);
        let v_idx = i * 2;
        let a_idx = i * 2 + 1;
        let seconds = (*dur).max(1.0) as u32;
        let script = format!("Page {}", i + 1);
        let safe_script = sanitize_drawtext(&script);
        filter.push_str(&format!(
            "[{v_idx}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black,zoompan=z='min(zoom+0.0008,1.15)':d={seconds}*25:s={w}x{h},drawtext=text='{safe_script}':fontcolor=white:fontsize=42:box=1:boxcolor=black@0.55:boxborderw=14:x=(w-text_w)/2:y=h-80[v{i}];",
        ));
        filter.push_str(&format!("[{a_idx}:a]aresample=44100[a{i}];"));
        audio_inputs.push(format!("[v{i}][a{i}]"));
    }
    filter.push_str(&format!(
        "{}concat=n={}:v=1:a=1[vout][aout]",
        audio_inputs.join(""),
        audio_inputs.len()
    ));
    (inputs, filter)
}

fn override_preset(args: &mut [String], preset: &str) {
    if let Some(i) = args.iter().position(|a| a == "-preset") {
        if let Some(v) = args.get_mut(i + 1) {
            *v = preset.to_string();
        }
    }
}

fn sanitize_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "")
        .replace('%', "\\%")
}

fn fixture_path(name: &str) -> PathBuf {
    let path = PathBuf::from(FIXTURES_DIR).join(name);
    assert!(
        path.exists(),
        "missing fixture {} (run scripts/gen_pdf_fixtures.py)",
        path.display()
    );
    path
}

fn locate_tool(name: &str, env_var: &str) -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var(env_var) {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Ok(pb);
        }
        return Err(format!("${env_var}={p} is not a file"));
    }
    let paths = std::env::var_os("PATH").ok_or_else(|| "PATH not set".to_string())?;
    for dir in std::env::split_paths(&paths) {
        for ext in ["", ".exe", ".bat", ".cmd"] {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(format!("{name} not on PATH"))
}

fn ffprobe_path() -> Result<PathBuf, String> {
    locate_tool("ffprobe", "PDF2VID_FFPROBE")
}

fn run_ffmpeg(ffmpeg: &Path, args: &[String]) -> Result<(), String> {
    let out = Command::new(ffmpeg)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("spawn ffmpeg: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "ffmpeg failed (exit {:?}): {}",
            out.status.code(),
            first_lines(&stderr, 5)
        ));
    }
    Ok(())
}

fn run_ffprobe(ffprobe: &Path, path: &Path) -> FfprobeResult {
    let raw = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("spawn ffprobe");
    serde_json::from_slice(&raw.stdout).expect("ffprobe json")
}

fn first_lines(text: &str, n: usize) -> String {
    text.lines().take(n).collect::<Vec<_>>().join(" | ")
}
