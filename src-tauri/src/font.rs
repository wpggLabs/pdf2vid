//! Cross-platform font discovery for FFmpeg `drawtext`.
//!
//! `drawtext` needs a concrete `fontfile` path on every platform; the
//! default `fontconfig` resolution only works on Linux with a populated
//! font cache. We probe a small list of known locations, prefer a TTF
//! (FFmpeg's drawtext handles `.ttf` more reliably than `.ttc` collection
//! files), and return a structured result so the caller can decide how
//! to react.
//!
//! The `work_dir` argument lets callers ask us to copy the resolved
//! font into a render-local directory and reference it by file name.
//! This is the safe shape for FFmpeg's filter parser: paths with colons
//! (Windows `C:\…`) or spaces break the option split.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Outcome of font discovery. Always return one of these — never a bare
/// `Option<PathBuf>` — so the caller can surface a clear warning to the
/// UI instead of a stringly-typed status message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FontResolution {
    /// True if a usable font file was located.
    pub found: bool,
    /// Absolute path to the source font we discovered on disk.
    pub source_path: Option<String>,
    /// Path that should be passed to FFmpeg. When `work_dir_copy` is
    /// used this is a safe filename inside the render work dir, which
    /// avoids the colon-in-path problem on Windows.
    pub render_path: Option<String>,
    /// Where `render_path` came from: `system` (raw system path, may
    /// contain colons/backslashes), `workdir` (safe filename copy), or
    /// `none` when nothing was found.
    pub render_kind: FontRenderKind,
    /// Human message — short, suitable for status bars.
    pub message: String,
    /// Platform-appropriate install hint when `found` is false.
    pub install_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FontRenderKind {
    System,
    Workdir,
    None,
}

/// Search the platform's known font directories for a usable TTF.
///
/// We always copy the resolved font into `work_dir` (creating it if
/// necessary) and return the safe filename. This is the portable shape
/// for FFmpeg: on Windows `C:\Windows\Fonts\arial.ttf` cannot be
/// inlined into a `drawtext=` filter without breaking the option
/// parser, and copying makes the path portable across platforms and
/// user installs.
///
/// When no font is available, returns a populated `FontResolution`
/// with `found: false` and an install hint so the caller can show a
/// proper warning.
pub fn resolve_font(work_dir: &Path) -> FontResolution {
    let candidates = default_candidates();

    for candidate in &candidates {
        let p = Path::new(candidate);
        if !p.is_file() {
            continue;
        }
        match stage_for_ffmpeg(p, work_dir) {
            Ok(staged) => {
                return FontResolution {
                    found: true,
                    source_path: Some(candidate.clone()),
                    // FFmpeg filter graphs choke on ':' in Windows drive paths
                    // (C:\...), so we always provide a colon-free render_path.
                    render_path: Some(staged.replace(':', "")),
                    render_kind: FontRenderKind::Workdir,
                    message: format!(
                        "Using font {}",
                        p.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                    ),
                    install_hint: None,
                };
            }
            Err(e) => {
                log::warn!("font candidate {} failed to stage: {e}", candidate);
                // Fall through to the next candidate rather than failing
                // the whole resolution — a write permission error on one
                // path shouldn't block the whole export.
            }
        }
    }

    FontResolution {
        found: false,
        source_path: None,
        render_path: None,
        render_kind: FontRenderKind::None,
        message: "No system font found; drawtext will be skipped".to_string(),
        install_hint: Some(install_hint_for_platform()),
    }
}

/// Copy the source font into the work dir under a stable, safe name
/// (`font.ttf`). Returns the absolute path of the staged file.
///
/// The `font.ttf` name has no spaces, colons, or backslashes — the
/// exact properties FFmpeg's filter parser likes.
fn stage_for_ffmpeg(source: &Path, work_dir: &Path) -> std::io::Result<String> {
    stage_font_for_render(source, work_dir)
}

/// Public copy of `stage_for_ffmpeg` for callers outside the `font`
/// module (e.g. the smoke example, integration tests, and any future
/// renderer). Always stages as `font.ttf` so the FFmpeg filter parser
/// sees a safe filename.
pub fn stage_font_for_render(source: &Path, work_dir: &Path) -> std::io::Result<String> {
    std::fs::create_dir_all(work_dir)?;
    let dest = work_dir.join("font.ttf");
    std::fs::copy(source, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}

/// Platform-ordered list of font candidates. The first existing file
/// wins. Order matters: a TTF always comes before a TTC because drawtext
/// handles TTF more reliably.
fn default_candidates() -> Vec<String> {
    if cfg!(target_os = "windows") {
        vec![
            r"C:\Windows\Fonts\arial.ttf".into(),
            r"C:\Windows\Fonts\segoeui.ttf".into(),
            r"C:\Windows\Fonts\consola.ttf".into(),
            r"C:\Windows\Fonts\calibri.ttf".into(),
            r"C:\Windows\Fonts\verdana.ttf".into(),
            r"C:\Windows\Fonts\tahoma.ttf".into(),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/System/Library/Fonts/Supplemental/Arial.ttf".into(),
            "/System/Library/Fonts/Helvetica.ttc".into(),
            "/Library/Fonts/Arial.ttf".into(),
            "/System/Library/Fonts/HelveticaNeue.ttc".into(),
            "/System/Library/Fonts/SFNS.ttf".into(),
        ]
    } else {
        vec![
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".into(),
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf".into(),
            "/usr/share/fonts/truetype/freefont/FreeSans.ttf".into(),
            "/usr/share/fonts/TTF/DejaVuSans.ttf".into(),
            "/usr/share/fonts/dejavu/DejaVuSans.ttf".into(),
            "/usr/share/fonts/liberation/LiberationSans-Regular.ttf".into(),
        ]
    }
}

fn install_hint_for_platform() -> String {
    if cfg!(target_os = "windows") {
        "Install a TrueType font under C:\\Windows\\Fonts (e.g. Arial or Segoe UI).".into()
    } else if cfg!(target_os = "macos") {
        "Reinstall macOS system fonts, or copy a .ttf into /Library/Fonts.".into()
    } else {
        "Install fonts via your package manager: e.g. `sudo apt install fonts-dejavu`.".into()
    }
}

/// Quote a render path so it is safe to embed inside the FFmpeg
/// `drawtext=` filter graph. The render path is expected to be a
/// safe local file (no spaces, no colons) because `resolve_font`
/// stages everything under `font.ttf`, but we still escape
/// backslashes defensively so a custom `--font` override from a
/// CI script can't break the filter.
pub fn escape_fontfile_for_filter(render_path: &str) -> String {
    render_path
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_fontfile_handles_colons_and_backslashes() {
        assert_eq!(
            escape_fontfile_for_filter(r"C:\Windows\Fonts\arial.ttf"),
            r"C\:\\Windows\\Fonts\\arial.ttf"
        );
        assert_eq!(escape_fontfile_for_filter("/tmp/font.ttf"), "/tmp/font.ttf");
        assert_eq!(escape_fontfile_for_filter("/tmp/it's.ttf"), "/tmp/its.ttf");
    }

    #[test]
    fn install_hint_matches_platform() {
        let hint = install_hint_for_platform();
        if cfg!(target_os = "windows") {
            assert!(hint.contains("C:\\Windows\\Fonts"));
        } else if cfg!(target_os = "macos") {
            assert!(hint.contains("/Library/Fonts"));
        } else {
            assert!(hint.to_lowercase().contains("fonts"));
        }
    }

    #[test]
    fn resolve_font_returns_structured_missing_on_empty_dir() {
        let work = std::env::temp_dir().join("pdf2vid-font-test-empty");
        let _ = std::fs::create_dir_all(&work);
        // We can't easily fake a system with no fonts from a unit test,
        // but we can at least assert the success path on the current
        // platform if a known font is present, and that the failure
        // shape is populated otherwise.
        let r = resolve_font(&work);
        if r.found {
            assert!(r.render_path.is_some());
            assert_eq!(r.render_kind, FontRenderKind::Workdir);
            assert!(r.install_hint.is_none());
        } else {
            assert!(r.render_path.is_none());
            assert_eq!(r.render_kind, FontRenderKind::None);
            assert!(r.install_hint.is_some());
        }
    }

    #[test]
    fn stage_for_ffmpeg_copies_into_work_dir() {
        // Use an existing system font if we can find one; otherwise
        // synthesize one to verify the copy mechanics. We do not
        // require any specific font to exist on the test host.
        let work = std::env::temp_dir().join("pdf2vid-font-stage-test");
        let _ = std::fs::create_dir_all(&work);

        // Make a tiny stand-in source font. We don't need a real TTF —
        // we're testing the staging primitive.
        let stand_in = work.join("stand-in.ttf");
        std::fs::write(&stand_in, b"fake-font-bytes").unwrap();
        let staged = stage_for_ffmpeg(&stand_in, &work).unwrap();
        let staged_path = std::path::PathBuf::from(&staged);
        assert!(staged_path.exists());
        assert_eq!(staged_path.file_name().unwrap(), "font.ttf");
        assert_eq!(
            std::fs::read(&staged_path).unwrap(),
            b"fake-font-bytes".to_vec()
        );
    }

    #[test]
    fn resolve_font_uses_safe_filename() {
        let work = std::env::temp_dir().join("pdf2vid-font-safe-name");
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();

        // The point of this test is the *shape* of the render path.
        // We don't require no-colon (Windows temp dirs always start
        // with `C:\`), we require that the final filename we hand to
        // FFmpeg is the safe `font.ttf` — the escape helper handles
        // any path-level colons/backslashes.
        let r = resolve_font(&work);
        if let Some(p) = r.render_path.as_deref() {
            assert!(
                std::path::Path::new(p).file_name().and_then(|n| n.to_str()) == Some("font.ttf"),
                "render_path should end in font.ttf: {p}"
            );
            // And the helper escapes the wrapping path safely.
            let escaped = escape_fontfile_for_filter(p);
            // Either there were no special chars to escape, or every
            // colon is now backslash-escaped.
            if p.contains(':') {
                assert!(
                    escaped.contains(r"\:"),
                    "expected escaped colon in {escaped}"
                );
            }
        }
    }
}
