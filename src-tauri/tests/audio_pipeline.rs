//! Real audio path integration test.
//!
//! What this test proves:
//!
//! 1. When edge-tts + Python are available, `edgetts::synthesize`
//!    actually produces a usable MP3 that ffmpeg can wrap into a
//!    scene. We measure the result with ffprobe (duration > 0, audio
//!    stream present, finite duration).
//! 2. When edge-tts is unavailable, the test does NOT fake success —
//!    it records the skip reason and stays ignored-by-default. The
//!    exact command to run with the dependency installed is documented
//!    in `docs/BUILD_LOG.md`.
//! 3. The render-engine test path (Phase 2's `smoke_export` and the
//!    new `pdf_pipeline` test) uses synthetic silent audio only. We
//!    do not mix TTS into render tests.
//!
//! Run with:
//!
//! ```bash
//! cargo test --test audio_pipeline -- --ignored --nocapture
//! ```
//!
//! On a host with `python -m edge_tts` available this produces a
//! real MP3 file under the temp dir and prints the duration. On a
//! host without edge-tts the test prints the skip reason and exits
//! cleanly — `cargo test --lib` (the fast path) skips it.

use base64::Engine;
use std::path::{Path, PathBuf};
use std::process::Command;

#[tokio::test]
#[ignore]
async fn edge_tts_produces_real_mp3_with_bounded_duration() {
    // Skip early with a printed reason when the host has no edge-tts.
    // We never pretend the test passed: the assertion block is gated
    // by the availability probe.
    let python = match pdf2vid_lib::edgetts::detect_python_with_edge_tts() {
        Some(p) => p,
        None => {
            eprintln!(
                "edge-tts not available on this host; install with `pip install edge-tts` to enable this test"
            );
            return;
        }
    };

    let req = pdf2vid_lib::edgetts::TtsRequest {
        text: "Hello, this is a smoke test of the edge TTS integration.".to_string(),
        voice: "en-US-AriaNeural".to_string(),
        rate: None,
        pitch: None,
    };
    let resp = pdf2vid_lib::edgetts::synthesize(req)
        .await
        .expect("edge-tts synthesize should succeed on a host with edge-tts installed");

    assert!(
        !resp.audio_base64.is_empty(),
        "audio base64 should not be empty"
    );
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(resp.audio_base64.as_bytes())
        .expect("base64 must decode");
    assert!(
        bytes.len() > 1000,
        "MP3 must be >1KB, got {} bytes",
        bytes.len()
    );

    // Persist the bytes so ffprobe can verify them.
    let work = std::env::temp_dir().join("pdf2vid-audio-pipeline-test");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("mkdir");
    let mp3_path = work.join("scene.mp3");
    std::fs::write(&mp3_path, &bytes).expect("write mp3");

    let ffmpeg = locate("ffmpeg").expect("ffmpeg on PATH");
    let ffprobe = locate("ffprobe").expect("ffprobe on PATH");

    // Re-encode to a stable wav so duration probe is reliable (some
    // edge-tts MP3s have padding that confuses duration parsing).
    let wav = work.join("scene.wav");
    run_ffmpeg(
        &ffmpeg,
        &[
            "-y".to_string(),
            "-i".to_string(),
            mp3_path.to_string_lossy().to_string(),
            "-ar".to_string(),
            "44100".to_string(),
            "-ac".to_string(),
            "1".to_string(),
            wav.to_string_lossy().to_string(),
        ],
    )
    .expect("ffmpeg mp3->wav re-encode");

    let probe = run_ffprobe(&ffprobe, &wav);
    let audios: Vec<_> = probe
        .streams
        .iter()
        .filter(|s| s.codec_type == "audio")
        .collect();
    assert_eq!(audios.len(), 1, "expected exactly one audio stream");
    let duration: f64 = probe
        .format
        .duration
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    assert!(
        duration.is_finite() && duration > 0.0 && duration < 30.0,
        "duration must be bounded and finite, got {duration}"
    );
    eprintln!(
        "edge-tts produced {} bytes mp3, ffmpeg reports {duration:.2}s",
        bytes.len()
    );
    let _ = python; // silence unused warning on the host variable
}

#[tokio::test]
#[ignore]
async fn voice_to_language_recognizes_spanish_voice() {
    // Cross-language smoke: verify the synthesizer maps a non-English
    // voice to the right language code so the fallback chain picks
    // the right TTS endpoint. We do not need edge-tts for this; the
    // mapping is pure.
    // We re-export the function here to assert behaviour on the
    // public API surface.
    let en = pdf2vid_lib::edgetts::voice_to_language("en-US-AriaNeural");
    let es = pdf2vid_lib::edgetts::voice_to_language("es-ES-ElviraNeural");
    let zh = pdf2vid_lib::edgetts::voice_to_language("zh-CN-XiaoxiaoNeural");
    assert_eq!(en, "en");
    assert_eq!(es, "es");
    assert_eq!(zh, "zh-CN");
}

#[derive(Debug, serde::Deserialize)]
struct FfprobeStream {
    codec_type: String,
}

#[derive(Debug, serde::Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct FfprobeResult {
    streams: Vec<FfprobeStream>,
    format: FfprobeFormat,
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
            stderr.lines().take(5).collect::<Vec<_>>().join(" | ")
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

fn locate(name: &str) -> Option<PathBuf> {
    let env_var = format!("PDF2VID_{}", name.to_uppercase());
    if let Ok(p) = std::env::var(&env_var) {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        for ext in ["", ".exe", ".bat", ".cmd"] {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
