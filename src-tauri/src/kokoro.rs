//! Kokoro local text-to-speech.
//!
//! Kokoro is a small (82M) Apache-2.0 open-weight TTS model that runs
//! well on a consumer GPU (or CPU) and covers 8 languages. Like the
//! edge-tts integration, we drive it through a Python subprocess so the
//! heavy ML runtime stays out of the Rust binary and users opt in by
//! installing the package:
//!
//! ```bash
//! pip install kokoro soundfile
//! ```
//!
//! First run downloads the model weights from Hugging Face (~330 MB).
//! If Python or the `kokoro` package is missing, `synthesize` returns a
//! clear error and the render pipeline falls back to edge-tts.

use base64::Engine;
use std::path::PathBuf;
use std::process::Stdio;

pub struct KokoroRequest {
    pub text: String,
    /// Kokoro voice id, e.g. `af_heart`, `bf_emma`, `jf_alpha`.
    pub voice: String,
    /// Kokoro language code (first letter of the voice family):
    /// `a` American English, `b` British, `e` Spanish, `f` French,
    /// `h` Hindi, `i` Italian, `j` Japanese, `p` Portuguese, `z` Chinese.
    pub lang_code: String,
    /// Narration speed multiplier (1.0 = normal).
    pub speed: f32,
}

#[derive(Debug)]
pub struct KokoroResponse {
    pub audio_base64: String,
    pub format: String,
}

/// The inline Python program that performs synthesis. Reads UTF-8 text
/// from a file (avoids arg-escaping issues), writes a WAV to `out`.
const SYNTH_SCRIPT: &str = r#"
import sys, numpy as np, soundfile as sf
text_path, out_path, voice, lang, speed = sys.argv[1:6]
speed = float(speed)
with open(text_path, encoding='utf-8') as f:
    text = f.read()
from kokoro import KPipeline
pipe = KPipeline(lang_code=lang)
def to_np(a):
    try:
        return a.detach().cpu().numpy()
    except Exception:
        return np.asarray(a)
chunks = [to_np(a) for _, _, a in pipe(text, voice=voice, speed=speed)]
data = np.concatenate(chunks) if chunks else np.zeros(1, dtype='float32')
sf.write(out_path, data, 24000)
"#;

/// Detect a Python interpreter that can import `kokoro`. Returns the
/// interpreter path, or `None` if unavailable.
pub fn detect_python_with_kokoro() -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };
    for cmd in candidates {
        let mut probe = std::process::Command::new(cmd);
        probe
            .args(["-c", "import kokoro"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            probe.creation_flags(CREATE_NO_WINDOW);
        }
        if let Ok(status) = probe.status() {
            if status.success() {
                return Some(which(cmd).unwrap_or_else(|| PathBuf::from(cmd)));
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

pub async fn synthesize(
    req: KokoroRequest,
    on_progress: crate::subprocess::ProgressFn<'_>,
) -> Result<KokoroResponse, String> {
    if req.lang_code.is_empty() {
        return Err(
            "Kokoro does not support the selected language. Use edge-tts or a cloud voice.".into(),
        );
    }
    let python = detect_python_with_kokoro().ok_or_else(|| {
        "Python with the kokoro package not found. Install with: pip install kokoro soundfile"
            .to_string()
    })?;

    let temp_dir = std::env::temp_dir().join("pdf2vid-kokoro");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Cannot create temp dir: {e}"))?;
    let stamp = format!(
        "{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    let text_path = temp_dir.join(format!("in-{stamp}.txt"));
    let out_path = temp_dir.join(format!("out-{stamp}.wav"));
    std::fs::write(&text_path, &req.text).map_err(|e| format!("Cannot write text: {e}"))?;

    let mut cmd = std::process::Command::new(&python);
    cmd.arg("-c")
        .arg(SYNTH_SCRIPT)
        .arg(&text_path)
        .arg(&out_path)
        .arg(&req.voice)
        .arg(&req.lang_code)
        .arg(format!("{:.2}", req.speed.clamp(0.5, 2.0)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let (status, _stdout, stderr) = crate::subprocess::run_with_progress(cmd, on_progress)
        .map_err(|e| format!("Failed to spawn kokoro: {e}"))?;
    let _ = std::fs::remove_file(&text_path);

    if !status.success() {
        let _ = std::fs::remove_file(&out_path);
        return Err(format!("kokoro failed: {}", first_lines(&stderr, 5)));
    }

    let bytes = std::fs::read(&out_path).map_err(|e| format!("Read output failed: {e}"))?;
    let _ = std::fs::remove_file(&out_path);
    if bytes.is_empty() {
        return Err("kokoro produced empty audio".into());
    }

    Ok(KokoroResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        format: "wav".into(),
    })
}

fn first_lines(text: &str, n: usize) -> String {
    text.lines().take(n).collect::<Vec<_>>().join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_lang_is_rejected_fast() {
        // Should not even attempt to spawn Python for an unsupported language.
        let rt = std::thread::spawn(|| {
            futures::executor::block_on(synthesize(
                KokoroRequest {
                    text: "hola".into(),
                    voice: "af_heart".into(),
                    lang_code: String::new(),
                    speed: 1.0,
                },
                &|_| {},
            ))
        })
        .join()
        .unwrap();
        assert!(rt.is_err());
        assert!(rt.unwrap_err().contains("does not support"));
    }

    #[test]
    fn detect_python_or_skip() {
        if let Some(p) = detect_python_with_kokoro() {
            assert!(p.exists() || p.to_string_lossy().contains("python"));
        }
    }
}
