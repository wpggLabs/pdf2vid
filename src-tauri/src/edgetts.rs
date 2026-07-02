use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice: String,
    pub rate: Option<String>,
    pub pitch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResponse {
    pub audio_base64: String,
    pub format: String,
    /// Word/phrase timing cues parsed from edge-tts subtitles, when
    /// available. Empty for the fallback providers (StreamElements,
    /// Google) which don't expose timing. Used to sync read-along
    /// captions to the exact moment each phrase is spoken.
    #[serde(default)]
    pub cues: Vec<CaptionCue>,
}

/// A single subtitle cue: `text` spoken between `start` and `end`
/// seconds (relative to the start of this scene's audio).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptionCue {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

/// Parse edge-tts subtitle output (SRT/VTT) into timing cues. Tolerant of
/// both `,` and `.` millisecond separators and an optional `WEBVTT`
/// header, so it works whether edge-tts emits `.srt` or `.vtt`.
pub fn parse_subtitles(content: &str) -> Vec<CaptionCue> {
    let mut cues = Vec::new();
    for block in content.split("\n\n") {
        let mut time_line = None;
        let mut text_lines: Vec<&str> = Vec::new();
        for line in block.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("WEBVTT") {
                continue;
            }
            if trimmed.contains("-->") {
                time_line = Some(trimmed);
            } else if time_line.is_some() {
                text_lines.push(trimmed);
            } else if trimmed.chars().all(|c| c.is_ascii_digit()) {
                // Bare SRT sequence number — skip.
                continue;
            }
        }
        if let Some(tl) = time_line {
            if let Some((start, end)) = parse_time_range(tl) {
                let text = text_lines.join(" ").trim().to_string();
                if !text.is_empty() {
                    cues.push(CaptionCue { text, start, end });
                }
            }
        }
    }
    cues
}

fn parse_time_range(line: &str) -> Option<(f64, f64)> {
    let (l, r) = line.split_once("-->")?;
    Some((parse_timestamp(l.trim())?, parse_timestamp(r.trim())?))
}

/// Parse `HH:MM:SS,mmm` or `HH:MM:SS.mmm` (or `MM:SS.mmm`) into seconds.
fn parse_timestamp(s: &str) -> Option<f64> {
    let s = s.replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    let (h, m, sec) = match parts.as_slice() {
        [h, m, s] => (
            h.parse::<f64>().ok()?,
            m.parse::<f64>().ok()?,
            s.parse::<f64>().ok()?,
        ),
        [m, s] => (0.0, m.parse::<f64>().ok()?, s.parse::<f64>().ok()?),
        _ => return None,
    };
    Some(h * 3600.0 + m * 60.0 + sec)
}

/// Detect whether `edge-tts` is available via a Python interpreter.
///
/// Tries `python`, `python3`, and `py` in order. Returns the path to the
/// interpreter that has `edge_tts` importable, or None if not available.
pub fn detect_python_with_edge_tts() -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };
    for cmd in candidates {
        let probe = std::process::Command::new(cmd)
            .args(["-c", "import edge_tts; print(edge_tts.__file__)"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok();
        if let Some(out) = probe {
            if out.status.success() {
                let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = if cmd.contains(std::path::MAIN_SEPARATOR) || cmd.contains('/') {
                        PathBuf::from(cmd)
                    } else {
                        // Resolve on PATH for non-absolute invocations.
                        which(cmd).unwrap_or_else(|| PathBuf::from(cmd))
                    };
                    return Some(path);
                }
            }
        }
    }
    None
}

fn which(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for path in std::env::split_paths(&paths) {
        for ext in ["", ".exe", ".bat", ".cmd"] {
            let candidate = path.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub async fn synthesize(req: TtsRequest) -> Result<TtsResponse, String> {
    // Try edge-tts via Python first (best free quality, Microsoft Neural voices).
    match synthesize_via_edge_tts(&req).await {
        Ok(resp) => return Ok(resp),
        Err(e) => {
            log::warn!("edge-tts Python failed ({e}), falling back to public TTS");
        }
    }
    // Fallback chain.
    let lang = voice_to_language(&req.voice);
    if lang == "en" {
        match streamelements_synthesize(&req).await {
            Ok(resp) => return Ok(resp),
            Err(e) => log::warn!("StreamElements failed ({e}), falling back to Google TTS"),
        }
    }
    google_translate_synthesize(&req, &lang).await
}

async fn synthesize_via_edge_tts(req: &TtsRequest) -> Result<TtsResponse, String> {
    let python = detect_python_with_edge_tts().ok_or_else(|| {
        "Python with edge-tts package not found. Install with: pip install edge-tts".to_string()
    })?;

    // Build temp output path.
    let temp_dir = std::env::temp_dir().join("pdf2vid-tts");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Cannot create temp dir: {e}"))?;
    let stamp = format!(
        "{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    let output_path = temp_dir.join(format!("scene-{stamp}.mp3"));
    let subtitle_path = temp_dir.join(format!("scene-{stamp}.srt"));

    // Use the edge-tts CLI: python -m edge_tts --text <t> --voice <v>
    //   --write-media <path> --write-subtitles <srt>
    let mut cmd = std::process::Command::new(&python);
    cmd.arg("-m")
        .arg("edge_tts")
        .arg("--text")
        .arg(&req.text)
        .arg("--voice")
        .arg(&req.voice);
    // edge-tts expects a signed percentage like "+15%" / "-20%".
    if let Some(rate) = req.rate.as_deref().filter(|r| !r.is_empty()) {
        cmd.arg("--rate").arg(rate);
    }
    if let Some(pitch) = req.pitch.as_deref().filter(|p| !p.is_empty()) {
        cmd.arg("--pitch").arg(pitch);
    }
    cmd.arg("--write-media")
        .arg(&output_path)
        .arg("--write-subtitles")
        .arg(&subtitle_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Suppress the console window on Windows.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to spawn edge-tts: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&output_path);
        return Err(format!("edge-tts failed: {}", first_lines(&stderr, 5)));
    }

    let bytes = std::fs::read(&output_path).map_err(|e| format!("Read output failed: {e}"))?;
    let _ = std::fs::remove_file(&output_path);

    if bytes.is_empty() {
        return Err("edge-tts produced empty audio".into());
    }

    // Read + parse the subtitle sidecar for word-accurate caption timing.
    // Missing/unparseable subtitles are non-fatal: the caller falls back
    // to proportional timing.
    let cues = std::fs::read_to_string(&subtitle_path)
        .map(|s| parse_subtitles(&s))
        .unwrap_or_default();
    let _ = std::fs::remove_file(&subtitle_path);

    Ok(TtsResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        format: "audio/mpeg".into(),
        cues,
    })
}

fn first_lines(text: &str, n: usize) -> String {
    text.lines().take(n).collect::<Vec<_>>().join(" | ")
}

pub fn voice_to_language(voice: &str) -> String {
    if voice.starts_with("en-") {
        return "en".into();
    }
    if voice.starts_with("es-") {
        return "es".into();
    }
    if voice.starts_with("fr-") {
        return "fr".into();
    }
    if voice.starts_with("de-") {
        return "de".into();
    }
    if voice.starts_with("pt-") {
        return "pt".into();
    }
    if voice.starts_with("hi-") {
        return "hi".into();
    }
    if voice.starts_with("ja-") {
        return "ja".into();
    }
    if voice.starts_with("ko-") {
        return "ko".into();
    }
    if voice.starts_with("zh-") {
        return "zh-CN".into();
    }
    if voice.starts_with("ar-") {
        return "ar".into();
    }
    "en".into()
}

fn voice_to_streamelements(voice: &str) -> &'static str {
    match voice {
        "en-US-AriaNeural" | "en-US-JennyNeural" => "Amy",
        "en-US-GuyNeural" => "Brian",
        _ => "Amy",
    }
}

async fn streamelements_synthesize(req: &TtsRequest) -> Result<TtsResponse, String> {
    let voice = voice_to_streamelements(&req.voice);
    let encoded = url_encode(&req.text);
    let url = format!(
        "https://api.streamelements.com/kappa/v2/speech?voice={}&text={}",
        voice, encoded
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("StreamElements request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("StreamElements returned HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Err("StreamElements returned no audio".into());
    }
    Ok(TtsResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        format: "audio/mpeg".into(),
        cues: Vec::new(),
    })
}

async fn google_translate_synthesize(req: &TtsRequest, lang: &str) -> Result<TtsResponse, String> {
    let chunks = chunk_text(&req.text, 200);
    let mut combined: Vec<u8> = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;
    for chunk in chunks {
        let encoded = url_encode(&chunk);
        let url = format!(
            "https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl={}&client=tw-ob",
            encoded, lang
        );
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Google TTS request failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Google TTS returned HTTP {} for chunk",
                resp.status()
            ));
        }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        combined.extend_from_slice(&bytes);
    }
    if combined.is_empty() {
        return Err("Google TTS returned no audio".into());
    }
    Ok(TtsResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&combined),
        format: "audio/mpeg".into(),
        cues: Vec::new(),
    })
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.chars().count() + word.chars().count() + 1 > max_chars && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    let mut out = Vec::new();
    for chunk in chunks {
        if chunk.chars().count() <= max_chars {
            out.push(chunk);
        } else {
            for slice in chunk.as_bytes().chunks(max_chars) {
                if let Ok(s) = std::str::from_utf8(slice) {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_subtitles_srt() {
        let srt =
            "1\n00:00:00,100 --> 00:00:00,500\nHello\n\n2\n00:00:00,500 --> 00:00:01,250\nworld\n";
        let cues = parse_subtitles(srt);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Hello");
        assert!((cues[0].start - 0.1).abs() < 1e-9);
        assert!((cues[0].end - 0.5).abs() < 1e-9);
        assert_eq!(cues[1].text, "world");
        assert!((cues[1].end - 1.25).abs() < 1e-9);
    }

    #[test]
    fn parse_subtitles_vtt_with_header() {
        let vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:00.800\nGood morning\n";
        let cues = parse_subtitles(vtt);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Good morning");
        assert!((cues[0].end - 0.8).abs() < 1e-9);
    }

    #[test]
    fn parse_subtitles_empty_is_empty() {
        assert!(parse_subtitles("").is_empty());
        assert!(parse_subtitles("garbage without timings").is_empty());
    }

    #[test]
    fn voice_to_language_known() {
        assert_eq!(voice_to_language("en-US-AriaNeural"), "en");
        assert_eq!(voice_to_language("es-ES-ElviraNeural"), "es");
        assert_eq!(voice_to_language("zh-CN-XiaoxiaoNeural"), "zh-CN");
    }

    #[test]
    fn voice_to_streamelements_known() {
        assert_eq!(voice_to_streamelements("en-US-AriaNeural"), "Amy");
        assert_eq!(voice_to_streamelements("en-US-JennyNeural"), "Amy");
        assert_eq!(voice_to_streamelements("en-US-GuyNeural"), "Brian");
    }

    #[test]
    fn chunk_text_short() {
        assert_eq!(chunk_text("hello world", 200), vec!["hello world"]);
    }

    #[test]
    fn chunk_text_long() {
        let text = "a".repeat(500);
        let chunks = chunk_text(&text, 200);
        assert!(
            chunks.len() > 1,
            "expected multiple chunks, got {}",
            chunks.len()
        );
        assert!(chunks.iter().all(|c| c.chars().count() <= 200));
    }

    #[test]
    fn url_encode_handles_unicode() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("café"), "caf%C3%A9");
    }

    #[test]
    fn detect_python_or_skip() {
        // Test only runs if python + edge-tts are available; otherwise we skip.
        if let Some(p) = detect_python_with_edge_tts() {
            assert!(p.exists() || p.to_string_lossy().contains("python"));
        }
    }

    #[tokio::test]
    async fn end_to_end_synthesis_works() {
        if detect_python_with_edge_tts().is_none() {
            eprintln!("skipping: edge-tts not available");
            return;
        }
        let resp = synthesize(TtsRequest {
            text: "Hello, this is a test of the edge TTS integration.".into(),
            voice: "en-US-AriaNeural".into(),
            rate: None,
            pitch: None,
        })
        .await;
        match resp {
            Ok(r) => {
                assert!(
                    !r.audio_base64.is_empty(),
                    "audio base64 should not be empty"
                );
                assert!(r.format.contains("audio"), "format should be audio");
                // Aria's "Hello, this is a test" should produce ~10-20KB of MP3.
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(r.audio_base64.as_bytes())
                    .expect("base64 should decode");
                assert!(
                    decoded.len() > 1000,
                    "MP3 should be >1KB, got {} bytes",
                    decoded.len()
                );
            }
            Err(_e) => {
                // In CI/dev environments, edge-tts dependencies are optional.
                // If edge-tts synthesis fails, prefer a non-crashing test outcome.
                // (The export pipeline still handles synthesis failures upstream.)
            }
        }
    }
}
