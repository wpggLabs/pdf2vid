//! Chatterbox Multilingual — premium local text-to-speech.
//!
//! Chatterbox is an MIT-licensed open-weight TTS model covering 23
//! languages with expressive, emotion-capable output. It benefits from a
//! GPU (an RTX-class card runs it comfortably). Like the other local
//! providers, it is driven through a Python subprocess so users opt in:
//!
//! ```bash
//! pip install chatterbox-tts torchaudio
//! ```
//!
//! First run downloads the model weights from Hugging Face. If Python or
//! the package is missing, `synthesize` returns a clear error and the
//! render pipeline falls back to edge-tts.

use base64::Engine;
use std::path::PathBuf;
use std::process::Stdio;

pub struct ChatterboxRequest {
    pub text: String,
    /// Chatterbox language id, e.g. `en`, `es`, `ja`, `zh`.
    pub language_id: String,
}

#[derive(Debug)]
pub struct ChatterboxResponse {
    pub audio_base64: String,
    pub format: String,
}

/// Inline Python program: loads the multilingual model (GPU if available),
/// synthesizes, and writes a WAV to `out`.
const SYNTH_SCRIPT: &str = r#"
import sys
text_path, out_path, lang = sys.argv[1], sys.argv[2], sys.argv[3]
with open(text_path, encoding='utf-8') as f:
    text = f.read()
import torch, torchaudio
from chatterbox.mtl_tts import ChatterboxMultilingualTTS
device = "cuda" if torch.cuda.is_available() else "cpu"
model = ChatterboxMultilingualTTS.from_pretrained(device=device)
wav = model.generate(text, language_id=lang)
torchaudio.save(out_path, wav.detach().cpu(), model.sr)
"#;

/// Detect a Python interpreter that can import `chatterbox`.
pub fn detect_python_with_chatterbox() -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };
    for cmd in candidates {
        let mut probe = std::process::Command::new(cmd);
        probe
            .args(["-c", "import chatterbox.mtl_tts"])
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

pub async fn synthesize(req: ChatterboxRequest) -> Result<ChatterboxResponse, String> {
    if req.language_id.is_empty() {
        return Err("Chatterbox: unsupported language".into());
    }
    let python = detect_python_with_chatterbox().ok_or_else(|| {
        "Python with the chatterbox package not found. Install with: pip install chatterbox-tts torchaudio"
            .to_string()
    })?;

    let temp_dir = std::env::temp_dir().join("pdf2vid-chatterbox");
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
        .arg(&req.language_id)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to spawn chatterbox: {e}"))?;
    let _ = std::fs::remove_file(&text_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&out_path);
        return Err(format!("chatterbox failed: {}", first_lines(&stderr, 5)));
    }

    let bytes = std::fs::read(&out_path).map_err(|e| format!("Read output failed: {e}"))?;
    let _ = std::fs::remove_file(&out_path);
    if bytes.is_empty() {
        return Err("chatterbox produced empty audio".into());
    }

    Ok(ChatterboxResponse {
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
    fn empty_language_is_rejected_fast() {
        let r = futures::executor::block_on(synthesize(ChatterboxRequest {
            text: "hello".into(),
            language_id: String::new(),
        }));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("unsupported language"));
    }

    #[test]
    fn detect_python_or_skip() {
        if let Some(p) = detect_python_with_chatterbox() {
            assert!(p.exists() || p.to_string_lossy().contains("python"));
        }
    }
}
