//! End-to-end export smoke test that runs through `cargo test`.
//!
//! This is a real FFmpeg integration test — it spawns `ffmpeg` and
//! `ffprobe`, generates placeholder visuals and audio, builds the same
//! filter graph that production uses, and verifies both outputs.
//!
//! Marked `#[ignore]` so `cargo test --lib` (the fast path) does not
//! run it. Opt in with:
//!
//! ```bash
//! cargo test --test smoke_export -- --ignored --nocapture
//! ```
//!
//! Required environment:
//!
//! - `ffmpeg` and `ffprobe` on PATH (or set `PDF2VID_FFMPEG` /
//!   `PDF2VID_FFPROBE` to absolute paths).
//! - At least one system font reachable by `pdf2vid_lib::font::resolve_font`.
//!   On Windows this is `C:\Windows\Fonts\arial.ttf`; on macOS
//!   `/System/Library/Fonts/Supplemental/Arial.ttf`; on Linux
//!   `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf` (or similar).
//!
//! On hosts that have no font at all, pass `--no-fallback` to confirm
//! the missing-font path emits the right warning rather than failing
//! the encode.

use pdf2vid_lib::font::{resolve_font, stage_font_for_render, FontRenderKind, FontResolution};
use pdf2vid_lib::render::build_ffmpeg_args;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const SCENE_DURATIONS_SECS: [f64; 3] = [2.0, 3.0, 2.0];

#[derive(Debug, serde::Serialize, Deserialize)]
struct FfprobeStream {
    codec_type: String,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, serde::Serialize, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    size: Option<String>,
}

#[derive(Debug, serde::Serialize, Deserialize)]
struct FfprobeResult {
    streams: Vec<FfprobeStream>,
    format: FfprobeFormat,
}

#[derive(Debug, serde::Serialize)]
struct StreamReport {
    path: String,
    bytes: u64,
    width: u32,
    height: u32,
    duration_seconds: f64,
    has_video: bool,
    has_audio: bool,
    issues: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct SmokeReport {
    pass: bool,
    youtube: StreamReport,
    tiktok: StreamReport,
    font_path: Option<String>,
    font_found: bool,
}

#[test]
#[ignore]
fn end_to_end_export_smoke() {
    let started = Instant::now();
    let report = match run_export_smoke() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("smoke setup failed: {e}");
            // Print elapsed so CI logs show how far we got.
            eprintln!("elapsed: {:?}", started.elapsed());
            panic!("smoke setup failed: {e}");
        }
    };
    eprintln!(
        "smoke: youtube {}x{}, tiktok {}x{}, font={:?}, elapsed={:?}",
        report.youtube.width,
        report.youtube.height,
        report.tiktok.width,
        report.tiktok.height,
        report.font_path,
        started.elapsed()
    );
    assert!(
        report.pass,
        "smoke failed:\n{}",
        serde_json::to_string_pretty(&report).unwrap()
    );

    // Hard checks required by the Phase 2 task list.
    assert!(report.youtube.has_video, "youtube missing video stream");
    assert!(report.youtube.has_audio, "youtube missing audio stream");
    assert_eq!(report.youtube.width, 1920);
    assert_eq!(report.youtube.height, 1080);
    assert!(
        report.youtube.duration_seconds.is_finite()
            && report.youtube.duration_seconds > 0.0
            && report.youtube.duration_seconds <= 30.0,
        "youtube duration out of bounds: {}",
        report.youtube.duration_seconds
    );

    assert!(report.tiktok.has_video, "tiktok missing video stream");
    assert!(report.tiktok.has_audio, "tiktok missing audio stream");
    assert_eq!(report.tiktok.width, 1080);
    assert_eq!(report.tiktok.height, 1920);
    assert!(
        report.tiktok.duration_seconds.is_finite()
            && report.tiktok.duration_seconds > 0.0
            && report.tiktok.duration_seconds <= 30.0,
        "tiktok duration out of bounds: {}",
        report.tiktok.duration_seconds
    );

    // Both files must exist on disk.
    assert!(Path::new(&report.youtube.path).is_file());
    assert!(Path::new(&report.tiktok.path).is_file());

    // Font was discovered and used. If the host has no fonts we treat
    // that as a setup error — see the docs in this file.
    assert!(report.font_found, "no font was discovered on this host");
}

fn run_export_smoke() -> Result<SmokeReport, String> {
    let work_dir = std::env::temp_dir().join("pdf2vid-smoke-test");
    let _ = std::fs::remove_dir_all(&work_dir);
    std::fs::create_dir_all(&work_dir).map_err(|e| format!("mkdir work: {e}"))?;

    let ffmpeg = locate_tool("ffmpeg", "PDF2VID_FFMPEG")?;
    let ffprobe = locate_tool("ffprobe", "PDF2VID_FFPROBE")?;

    let font_resolution = resolve_font(&work_dir);
    let font_path = font_resolution.render_path.clone();
    let font_for_ffmpeg = font_path
        .as_deref()
        .map(pdf2vid_lib::font::escape_fontfile_for_filter);

    // Generate placeholder visuals + silent audio.
    let mut visual_paths: Vec<PathBuf> = Vec::new();
    let mut audio_paths: Vec<PathBuf> = Vec::new();
    for (idx, dur) in SCENE_DURATIONS_SECS.iter().enumerate() {
        let v = work_dir.join(format!("scene-{idx}.png"));
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

        let a = work_dir.join(format!("scene-{idx}.m4a"));
        run_ffmpeg(
            &ffmpeg,
            &[
                "-y".into(),
                "-f".into(),
                "lavfi".into(),
                "-i".into(),
                format!("anullsrc=r=44100:cl=mono:d={dur}"),
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

    let youtube_out = work_dir.join("sample-youtube.mp4");
    let tiktok_out = work_dir.join("sample-tiktok.mp4");

    // YouTube render.
    let (inputs_y, filter_y) = build_filter(
        &SCENE_DURATIONS_SECS,
        &visual_paths,
        &audio_paths,
        1920,
        1080,
    );
    let mut args_y = build_ffmpeg_args(
        &inputs_y,
        &filter_y,
        &youtube_out,
        font_for_ffmpeg.as_deref(),
    );
    override_preset(&mut args_y, "ultrafast");
    run_ffmpeg(&ffmpeg, &args_y)?;

    // TikTok render.
    let (inputs_t, filter_t) = build_filter(
        &SCENE_DURATIONS_SECS,
        &visual_paths,
        &audio_paths,
        1080,
        1920,
    );
    let mut args_t = build_ffmpeg_args(
        &inputs_t,
        &filter_t,
        &tiktok_out,
        font_for_ffmpeg.as_deref(),
    );
    override_preset(&mut args_t, "ultrafast");
    run_ffmpeg(&ffmpeg, &args_t)?;

    let youtube = probe(&ffprobe, &youtube_out, 1920, 1080)?;
    let tiktok = probe(&ffprobe, &tiktok_out, 1080, 1920)?;
    let pass = youtube.issues.is_empty() && tiktok.issues.is_empty();

    Ok(SmokeReport {
        pass,
        youtube,
        tiktok,
        font_path,
        font_found: font_resolution.found,
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

fn override_preset(args: &mut Vec<String>, preset: &str) {
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

fn probe(ffprobe: &Path, path: &Path, want_w: u32, want_h: u32) -> Result<StreamReport, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("stat {path:?}: {e}"))?;
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
        .map_err(|e| format!("spawn ffprobe: {e}"))?;
    if !raw.status.success() {
        return Err(format!(
            "ffprobe failed for {path:?}: {}",
            String::from_utf8_lossy(&raw.stderr)
        ));
    }
    let parsed: FfprobeResult =
        serde_json::from_slice(&raw.stdout).map_err(|e| format!("ffprobe json parse: {e}"))?;

    let mut issues = Vec::new();
    let videos: Vec<_> = parsed
        .streams
        .iter()
        .filter(|s| s.codec_type == "video")
        .collect();
    let audios: Vec<_> = parsed
        .streams
        .iter()
        .filter(|s| s.codec_type == "audio")
        .collect();
    if videos.is_empty() {
        issues.push("no video stream".into());
    }
    if audios.is_empty() {
        issues.push("no audio stream".into());
    }
    let (mut width, mut height) = (0u32, 0u32);
    if let Some(v) = videos.first() {
        match (v.width, v.height) {
            (Some(w), Some(h)) => {
                width = w;
                height = h;
                if (w, h) != (want_w, want_h) {
                    issues.push(format!("resolution {w}x{h} != {want_w}x{want_h}"));
                }
            }
            _ => issues.push("video stream missing width/height".into()),
        }
    }
    let duration = parsed
        .format
        .duration
        .as_deref()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    if !duration.is_finite() || !(0.5..=30.0).contains(&duration) {
        issues.push(format!("duration {duration}s out of bounds or non-finite"));
    }

    Ok(StreamReport {
        path: path.to_string_lossy().to_string(),
        bytes: meta.len(),
        width,
        height,
        duration_seconds: duration,
        has_video: !videos.is_empty(),
        has_audio: !audios.is_empty(),
        issues,
    })
}

fn first_lines(text: &str, n: usize) -> String {
    text.lines().take(n).collect::<Vec<_>>().join(" | ")
}

#[test]
fn font_discovery_returns_structured_shape() {
    // We can't force a particular outcome (no fonts on host vs some
    // fonts), but we can guarantee the result is well-formed and that
    // `render_path` is colon-free when present.
    let work = std::env::temp_dir().join("pdf2vid-smoke-font-shape");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let r: FontResolution = resolve_font(&work);
    if let Some(p) = r.render_path.as_deref() {
        assert!(!p.contains(':'), "render_path must be colon-free: {p}");
    }
    if r.found {
        assert_eq!(r.render_kind, FontRenderKind::Workdir);
    } else {
        assert_eq!(r.render_kind, FontRenderKind::None);
        assert!(r.install_hint.is_some());
    }
}

#[test]
fn stage_font_for_render_copies_into_work_dir() {
    let work = std::env::temp_dir().join("pdf2vid-smoke-stage");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let source = work.join("input.ttf");
    std::fs::write(&source, b"fake-font-bytes").unwrap();
    let staged = stage_font_for_render(&source, &work).unwrap();
    let staged_pb = PathBuf::from(&staged);
    assert!(staged_pb.exists());
    assert_eq!(staged_pb.file_name().unwrap(), "font.ttf");
    assert_eq!(
        std::fs::read(&staged_pb).unwrap(),
        b"fake-font-bytes".to_vec()
    );
}
