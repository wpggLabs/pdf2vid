//! Generate manual QA outputs from real PDF fixtures.
//!
//! Run with: `cargo run --example qa_export`
//!
//! Outputs land in `docs/manual_qa/`:
//!
//!   - `clean-youtube.mp4`
//!   - `mixed-youtube.mp4`
//!   - `non-english-youtube.mp4`
//!
//! The example deliberately uses the production render path: it
//! imports the lib's font discovery + `build_ffmpeg_args` + filter
//! graph. Voice is replaced with silent audio (the QA outputs are
//! for visual proof; voice QA goes through `tests/audio_pipeline.rs`).
//!
//! After running, copy `docs/manual_qa/qa-report.json` into the
//! BUILD_LOG entry for Phase 2.5.

use pdf2vid_lib::font::resolve_font;
use pdf2vid_lib::render::build_ffmpeg_args;
use pdf2vid_lib::types::{Project, Scene};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURES_DIR: &str = "../fixtures";
const OUT_DIR: &str = "../docs/manual_qa";

#[derive(Serialize)]
struct QaEntry {
    fixture: String,
    output_path: String,
    bytes: u64,
    pages_imported: u32,
    pages_skipped: Vec<u32>,
    resolution: (u32, u32),
    duration_seconds: f64,
    has_video: bool,
    has_audio: bool,
    font_path: Option<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct QaReport {
    host_platform: String,
    ffmpeg_version: Option<String>,
    font_found: bool,
    entries: Vec<QaEntry>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("qa_export failed: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let fixtures_root = PathBuf::from(FIXTURES_DIR);
    let out_dir = PathBuf::from(OUT_DIR);
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("mkdir out: {e}"))?;

    let ffmpeg = locate("ffmpeg")?;
    let ffprobe = locate("ffprobe")?;

    let work = std::env::temp_dir().join("pdf2vid-qa");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| format!("mkdir work: {e}"))?;

    let font_resolution = resolve_font(&work);
    let font_path = font_resolution.render_path.clone();
    let font_for_ffmpeg = font_path
        .as_deref()
        .map(pdf2vid_lib::font::escape_fontfile_for_filter);

    let fixtures: &[&str] = &[
        "clean-text-3page.pdf",
        "mixed-blank-page.pdf",
        "non-english-3page.pdf",
    ];

    let mut entries = Vec::new();
    for fixture in fixtures {
        let pdf = fixtures_root.join(fixture);
        if !pdf.exists() {
            eprintln!("skip {}: missing", pdf.display());
            continue;
        }
        let project = build_project_for_qa(fixture, &pdf)?;
        let warnings = vec![
            format!("Skipped pages: {:?}", project.skipped_pages),
            format!("Render fallback used: {}", !font_resolution.found),
        ];
        let out = out_dir.join(format!(
            "{}-youtube.mp4",
            Path::new(fixture)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("out")
        ));
        render(&ffmpeg, &project, font_for_ffmpeg.as_deref(), &out)?;
        let probe = probe(&ffprobe, &out)?;
        let duration = probe
            .format
            .duration
            .as_deref()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let meta = std::fs::metadata(&out).map_err(|e| format!("stat: {e}"))?;
        let video = probe
            .streams
            .iter()
            .find(|s| s.codec_type == "video")
            .map(|s| (s.width.unwrap_or(0), s.height.unwrap_or(0)))
            .unwrap_or((0, 0));
        let has_video = probe.streams.iter().any(|s| s.codec_type == "video");
        let has_audio = probe.streams.iter().any(|s| s.codec_type == "audio");
        entries.push(QaEntry {
            fixture: fixture.to_string(),
            output_path: out.to_string_lossy().to_string(),
            bytes: meta.len(),
            pages_imported: project.scenes.len() as u32,
            pages_skipped: project.skipped_pages.clone(),
            resolution: video,
            duration_seconds: duration,
            has_video,
            has_audio,
            font_path: font_path.clone(),
            warnings: warnings.clone(),
        });
        println!(
            "  wrote {} ({}x{}, {} pages, {} skipped, {:.2}s)",
            out.display(),
            video.0,
            video.1,
            project.scenes.len(),
            project.skipped_pages.len(),
            duration
        );
    }

    let report = QaReport {
        host_platform: std::env::consts::OS.to_string(),
        ffmpeg_version: ffmpeg_version(&ffmpeg),
        font_found: font_resolution.found,
        entries,
    };
    let report_path = out_dir.join("qa-report.json");
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).map_err(|e| format!("report json: {e}"))?,
    )
    .map_err(|e| format!("write report: {e}"))?;
    println!("  wrote {}", report_path.display());
    Ok(())
}

fn build_project_for_qa(name: &str, path: &Path) -> Result<Project, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let pages_text = pdf_extract::extract_text_from_mem_by_pages(&bytes)
        .map_err(|e| format!("pdf-extract {}: {e}", path.display()))?;
    let mut scenes = Vec::new();
    let mut skipped = Vec::new();
    for (idx, text) in pages_text.into_iter().enumerate() {
        let trimmed = text.trim();
        if trimmed.len() < 5 {
            skipped.push((idx + 1) as u32);
            continue;
        }
        let duration = std::cmp::max(
            4u32,
            ((trimmed.split_whitespace().count() as f64) / 2.5).ceil() as u32,
        );
        scenes.push(Scene {
            id: format!("page-{}", idx + 1),
            page: (idx + 1) as u32,
            title: trimmed
                .chars()
                .take(42)
                .collect::<String>()
                .trim()
                .to_string(),
            script: trimmed.to_string(),
            translated_script: None,
            duration,
            selected: true,
            thumbnail: String::new(),
        });
    }
    if scenes.is_empty() {
        return Err(format!(
            "{} has no text-bearing pages; QA export aborted",
            path.display()
        ));
    }
    Ok(Project {
        name: name.to_string(),
        source_name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("input.pdf")
            .to_string(),
        scenes,
        language: "English (US)".to_string(),
        translation_provider: "marian".to_string(),
        voice_provider: "edge".to_string(),
        voice: "en-US-JennyNeural".to_string(),
        output_you_tube: true,
        output_tik_tok: false,
        skipped_pages: skipped,
        voice_speed: 100,
    })
}

fn render(
    ffmpeg: &Path,
    project: &Project,
    font_path: Option<&str>,
    output: &Path,
) -> Result<(), String> {
    let work = std::env::temp_dir().join(format!(
        "pdf2vid-qa-{}",
        output.file_stem().and_then(|s| s.to_str()).unwrap_or("out")
    ));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| format!("mkdir: {e}"))?;

    let mut visual_paths = Vec::new();
    let mut audio_paths = Vec::new();
    let per_scene: f64 = project
        .scenes
        .iter()
        .map(|s| s.duration as f64)
        .sum::<f64>()
        / project.scenes.len().max(1) as f64;
    for idx in 0..project.scenes.len() {
        let v = work.join(format!("scene-{idx}.png"));
        run_ffmpeg(
            ffmpeg,
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
            ffmpeg,
            &[
                "-y".into(),
                "-f".into(),
                "lavfi".into(),
                "-i".into(),
                format!("anullsrc=r=44100:cl=mono:d={per_scene}"),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "96k".into(),
                "-shortest".into(),
                a.to_string_lossy().to_string(),
            ],
        )?;
        audio_paths.push(a);
    }

    let (w, h) = (1920u32, 1080u32);
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
    let mut args = build_ffmpeg_args(&inputs, &filter, &output.to_path_buf(), font_path);
    if let Some(i) = args.iter().position(|a| a == "-preset") {
        if let Some(v) = args.get_mut(i + 1) {
            *v = "ultrafast".into();
        }
    }
    run_ffmpeg(ffmpeg, &args)?;
    Ok(())
}

fn build_filter(
    scenes: &[f64],
    visuals: &[PathBuf],
    audios: &[PathBuf],
    w: u32,
    h: u32,
) -> (Vec<String>, String) {
    let mut inputs = Vec::new();
    let mut filter = String::new();
    let mut audio_inputs = Vec::new();
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

fn sanitize_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "")
        .replace('%', "\\%")
}

#[derive(serde::Deserialize)]
struct FfprobeStream {
    codec_type: String,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(serde::Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(serde::Deserialize)]
struct FfprobeResult {
    streams: Vec<FfprobeStream>,
    format: FfprobeFormat,
}

fn probe(ffprobe: &Path, path: &Path) -> Result<FfprobeResult, String> {
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
        .map_err(|e| format!("ffprobe spawn: {e}"))?;
    if !raw.status.success() {
        return Err(format!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&raw.stderr)
        ));
    }
    serde_json::from_slice(&raw.stdout).map_err(|e| format!("ffprobe json: {e}"))
}

fn run_ffmpeg(ffmpeg: &Path, args: &[String]) -> Result<(), String> {
    let out = Command::new(ffmpeg)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("ffmpeg spawn: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "ffmpeg failed: {}",
            stderr.lines().take(5).collect::<Vec<_>>().join(" | ")
        ));
    }
    Ok(())
}

fn locate(name: &str) -> Result<PathBuf, String> {
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

fn ffmpeg_version(ffmpeg: &Path) -> Option<String> {
    let out = Command::new(ffmpeg)
        .arg("-version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
}
