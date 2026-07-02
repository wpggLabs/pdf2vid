//! Argos Translate — offline neural machine translation.
//!
//! Argos is an open-source (MIT) offline translator built on OpenNMT /
//! CTranslate2. Like the other local providers, we drive it through a
//! Python subprocess so the ML runtime stays out of the Rust binary and
//! users opt in by installing the package:
//!
//! ```bash
//! pip install argostranslate
//! ```
//!
//! The first translation for a given language pair downloads a small
//! language package (~100 MB) from the Argos index. If Python or the
//! package is missing, `translate` returns a clear error and the render
//! pipeline falls back to the source text (with a structured warning).

use std::path::PathBuf;
use std::process::Stdio;

/// Inline Python program: installs the language pair on first use, then
/// translates UTF-8 text read from a file and writes the result to stdout.
const TRANSLATE_SCRIPT: &str = r#"
import sys
text_path, from_code, to_code = sys.argv[1], sys.argv[2], sys.argv[3]
with open(text_path, encoding='utf-8') as f:
    text = f.read()
import argostranslate.package as package
import argostranslate.translate as translate
def has_pair(fc, tc):
    langs = translate.get_installed_languages()
    fl = next((l for l in langs if l.code == fc), None)
    tl = next((l for l in langs if l.code == tc), None)
    return bool(fl and tl and fl.get_translation(tl))
if not has_pair(from_code, to_code):
    package.update_package_index()
    avail = package.get_available_packages()
    p = next((x for x in avail if x.from_code == from_code and x.to_code == to_code), None)
    if p is None:
        sys.stderr.write('No Argos language package for %s->%s' % (from_code, to_code))
        sys.exit(3)
    package.install_from_path(p.download())
sys.stdout.write(translate.translate(text, from_code, to_code))
"#;

/// Detect a Python interpreter that can import `argostranslate`.
pub fn detect_python_with_argos() -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };
    for cmd in candidates {
        let mut probe = std::process::Command::new(cmd);
        probe
            .args(["-c", "import argostranslate.translate"])
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

/// Translate `text` from `from_code` to `to_code` (ISO 639-1 codes, e.g.
/// `en`, `es`, `ja`). Returns the translated text.
pub async fn translate(
    from_code: &str,
    to_code: &str,
    text: &str,
    on_progress: crate::subprocess::ProgressFn<'_>,
) -> Result<String, String> {
    if from_code.is_empty() || to_code.is_empty() {
        return Err("Argos: unsupported language pair".into());
    }
    if from_code == to_code {
        return Ok(text.to_string());
    }
    let python = detect_python_with_argos().ok_or_else(|| {
        "Python with the argostranslate package not found. Install with: pip install argostranslate"
            .to_string()
    })?;

    let temp_dir = std::env::temp_dir().join("pdf2vid-argos");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Cannot create temp dir: {e}"))?;
    let text_path = temp_dir.join(format!(
        "in-{}-{}.txt",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    std::fs::write(&text_path, text).map_err(|e| format!("Cannot write text: {e}"))?;

    let mut cmd = std::process::Command::new(&python);
    cmd.arg("-c")
        .arg(TRANSLATE_SCRIPT)
        .arg(&text_path)
        .arg(from_code)
        .arg(to_code)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let (status, stdout, stderr) = crate::subprocess::run_with_progress(cmd, on_progress)
        .map_err(|e| format!("Failed to spawn argostranslate: {e}"))?;
    let _ = std::fs::remove_file(&text_path);

    if !status.success() {
        return Err(format!("Argos failed: {}", first_lines(&stderr, 5)));
    }
    let out = stdout.trim().to_string();
    if out.is_empty() {
        return Err("Argos produced empty output".into());
    }
    Ok(out)
}

fn first_lines(text: &str, n: usize) -> String {
    text.lines().take(n).collect::<Vec<_>>().join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pair_is_rejected() {
        let r = futures::executor::block_on(translate("", "es", "hola", &|_| {}));
        assert!(r.is_err());
    }

    #[test]
    fn same_language_is_identity() {
        let r = futures::executor::block_on(translate("en", "en", "hello", &|_| {})).unwrap();
        assert_eq!(r, "hello");
    }

    #[test]
    fn detect_python_or_skip() {
        if let Some(p) = detect_python_with_argos() {
            assert!(p.exists() || p.to_string_lossy().contains("python"));
        }
    }
}
