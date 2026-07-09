//! OCR fallback for image-only PDF pages.
//!
//! `pdfjs-dist` only extracts *selectable* text. Scanned / image-based pages
//! return no text, which would otherwise be skipped entirely. When that
//! happens the frontend renders the page to a PNG and asks us to OCR it.
//!
//! We use **RapidOCR** (an ONNX-runtime based engine) because it installs
//! entirely through `pip` — no separate system binary, no admin, no package
//! manager. The engine lives in a **dedicated venv under the app's data
//! directory**, never the user's global site-packages: global installs break
//! on PEP 668 "externally managed" Pythons, conda environments, and Microsoft
//! Store Python stubs, and polluting the user's Python is rude regardless.

use base64::Engine as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::subprocess::hide_window;

/// Find a usable system Python interpreter (only needed to bootstrap the
/// venv; all OCR work runs through the venv's own interpreter).
fn detect_python() -> Option<PathBuf> {
    for name in ["python", "python3", "py"] {
        if Command::new(name)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(PathBuf::from(name));
        }
    }
    None
}

/// The Python interpreter inside our dedicated venv.
fn venv_python(venv_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    }
}

fn imports_ok(python: &Path) -> bool {
    Command::new(python)
        .args(["-c", "import rapidocr_onnxruntime, PIL"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Ensure the OCR venv exists and has `rapidocr-onnxruntime` + `pillow`
/// installed. Idempotent; cheap once everything is in place.
fn ensure_ocr_env(venv_dir: &Path) -> Result<(), String> {
    let vpy = venv_python(venv_dir);

    if !vpy.exists() {
        let system = detect_python().ok_or_else(|| {
            "OCR unavailable: Python is not installed. Install Python 3 to enable \
             scanned-PDF reading."
                .to_string()
        })?;
        let mut cmd = Command::new(&system);
        cmd.args(["-m", "venv"]).arg(venv_dir);
        hide_window(&mut cmd);
        let out = cmd
            .output()
            .map_err(|e| format!("OCR setup failed to create venv: {e}"))?;
        if !out.status.success() || !vpy.exists() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!(
                "OCR setup could not create a Python venv: {}",
                stderr.lines().last().unwrap_or("unknown error").trim()
            ));
        }
    }

    if imports_ok(&vpy) {
        return Ok(());
    }

    // First-time install into the venv. `--no-input` stops pip from ever
    // prompting (we run non-interactively). Use `output()` so the pipes are
    // fully drained — `status()` with a piped stream can deadlock once the
    // pipe buffer fills up.
    let mut cmd = Command::new(&vpy);
    cmd.args([
        "-m",
        "pip",
        "install",
        "--quiet",
        "--no-input",
        "rapidocr-onnxruntime",
        "pillow",
    ]);
    hide_window(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("OCR setup failed to launch pip: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let detail = stderr.lines().last().unwrap_or("").trim().to_string();
        return Err(format!(
            "Could not auto-install the OCR engine (rapidocr-onnxruntime). \
             Ensure network access is available. {detail}"
        ));
    }
    Ok(())
}

/// True when the OCR engine is already installed (no install attempted).
/// Used for status reporting so we don't trigger a download just to check.
pub fn ocr_available(venv_dir: &Path) -> bool {
    let vpy = venv_python(venv_dir);
    vpy.exists() && imports_ok(&vpy)
}

/// Install the OCR engine if it isn't present. Safe to call from startup;
/// failures are logged, not fatal.
pub fn ensure_ocr_installed(venv_dir: &Path) {
    if ocr_available(venv_dir) {
        return;
    }
    if let Err(e) = ensure_ocr_env(venv_dir) {
        log::warn!("OCR auto-install skipped: {e}");
    } else {
        log::info!("OCR engine installed successfully");
    }
}

/// OCR a base64-encoded PNG data URL and return the recognized text.
///
/// `data` is a `data:image/png;base64,...` data URL (as produced by the
/// browser's `canvas.toDataURL`). Returns an empty string when no text is
/// detected. Returns an `Err` when the OCR stack is unavailable.
pub fn ocr_png_data_url(venv_dir: &Path, data_url: &str) -> Result<String, String> {
    let b64 = data_url
        .split_once("base64,")
        .map(|(_, b)| b)
        .unwrap_or(data_url);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| format!("OCR: invalid image data: {e}"))?;

    ensure_ocr_env(venv_dir)?;
    let python = venv_python(venv_dir);

    let tmp = std::env::temp_dir().join("pdf2vid-ocr");
    let _ = std::fs::create_dir_all(&tmp);
    let img_path = tmp.join(format!("{}.png", uuid_simple()));
    {
        let mut f =
            std::fs::File::create(&img_path).map_err(|e| format!("OCR: write image: {e}"))?;
        f.write_all(&bytes)
            .map_err(|e| format!("OCR: write image: {e}"))?;
    }

    // RapidOCR returns a list of [box, text, score]; we join the texts.
    let script = format!(
        "import sys, json\n\
         try:\n\
             from PIL import Image\n\
             import numpy as np\n\
             from rapidocr_onnxruntime import RapidOCR\n\
             img = np.array(Image.open(r'{img}').convert('RGB'))\n\
             engine = RapidOCR()\n\
             result, _ = engine(img)\n\
             texts = [t[1] for t in (result or [])]\n\
             sys.stdout.write(json.dumps({{'text': '\\n'.join(texts)}}))\n\
         except Exception as e:\n\
             sys.stderr.write('ocr error: ' + str(e) + '\\n')\n\
             sys.exit(2)\n",
        img = img_path.display(),
    );

    let mut cmd = Command::new(&python);
    cmd.arg("-c").arg(&script);
    hide_window(&mut cmd);

    // Always remove the temp image, whether OCR succeeds or fails.
    let result = (|| {
        let output = cmd
            .output()
            .map_err(|e| format!("OCR: Python execution failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr
                .lines()
                .find(|l| l.contains("ocr error") || l.contains("Error"))
                .unwrap_or("OCR engine failed");
            return Err(format!("OCR failed: {msg}"));
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let parsed = serde_json::from_str::<serde_json::Value>(text.trim())
            .ok()
            .and_then(|v| {
                v.get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();
        Ok(parsed.trim().to_string())
    })();
    let _ = std::fs::remove_file(&img_path);
    result
}

/// Cheap unique-ish name for temp files (no external uuid dependency).
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}-{}", n, std::process::id())
}
